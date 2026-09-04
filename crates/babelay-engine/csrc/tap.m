// Core Audio Process Tap 심. macOS 14.2+.
#import <Foundation/Foundation.h>
#import <CoreAudio/CoreAudio.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>

typedef void (*babelay_cb)(const float *, uint32_t, uint32_t, double, void *);

typedef struct {
    AudioObjectID tap;
    AudioObjectID aggregate;
    AudioDeviceIOProcID proc;
    babelay_cb cb;
    void *user;
    uint32_t channels;
    double rate;
    bool interleaved;
    float *scratch;      // 논인터리브 입력을 인터리브할 때만 사용
    size_t scratch_cap;  // 샘플 개수
    // ARC 가 C 구조체 필드의 소유를 못 잡으므로 CFBridgingRetain/Release 로 수동 소유한다.
    void *queue;     // dispatch_queue_t — 리스너·재생성·정지가 직렬로 도는 큐
    void *listener;  // AudioObjectPropertyListenerBlock
    void *out_uid;   // NSString* — 현재 집계 장치가 물고 있는 기본 출력 UID
} tap_handle;

void babelay_tap_stop(void *handle);

static const AudioObjectPropertyAddress kDefaultOutputAddr = {
    kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyElementMain};

static OSStatus default_output_uid(NSString **uid) {
    AudioObjectPropertyAddress addr = kDefaultOutputAddr;
    AudioObjectID dev = 0;
    UInt32 size = sizeof(dev);
    OSStatus st = AudioObjectGetPropertyData(kAudioObjectSystemObject, &addr, 0, NULL, &size, &dev);
    if (st != noErr) return st;
    if (dev == kAudioObjectUnknown) return kAudioHardwareBadDeviceError;
    addr.mSelector = kAudioDevicePropertyDeviceUID;
    CFStringRef cf = NULL;
    size = sizeof(cf);
    st = AudioObjectGetPropertyData(dev, &addr, 0, NULL, &size, &cf);
    if (st != noErr) return st;
    *uid = (__bridge_transfer NSString *)cf;
    return noErr;
}

static void set_out_uid(tap_handle *h, NSString *uid) {
    if (h->out_uid) CFBridgingRelease(h->out_uid);
    h->out_uid = uid ? (void *)CFBridgingRetain(uid) : NULL;
}

static OSStatus create_tap(AudioObjectID *tapOut, CATapDescription **descOut) {
    CATapDescription *desc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
    desc.name = @"Babelay";
    desc.privateTap = YES;
    desc.muteBehavior = CATapUnmuted;
    OSStatus st = AudioHardwareCreateProcessTap(desc, tapOut);
    if (st == noErr && descOut) *descOut = desc;
    return st;
}

// 집계 장치·IOProc 만 닫는다. 탭과 콜백은 남긴다(재생성용).
static void close_aggregate(tap_handle *h) {
    if (h->proc) {
        AudioDeviceStop(h->aggregate, h->proc);
        AudioDeviceDestroyIOProcID(h->aggregate, h->proc);
        h->proc = NULL;
    }
    if (h->aggregate) {
        AudioHardwareDestroyAggregateDevice(h->aggregate);
        h->aggregate = 0;
    }
    set_out_uid(h, nil);
}

// 현재 기본 출력 장치로 집계 장치를 만들고 탭 포맷을 읽고 IOProc 을 시작한다.
// 0 이 아니면 실패 — 만든 것은 되돌렸다. (-2 = 포맷 미지원)
static int open_aggregate(tap_handle *h, NSString *tapUUID) {
    NSString *outUID = nil;
    OSStatus st = default_output_uid(&outUID);
    if (st != noErr) return (int)st;

    NSDictionary *aggDesc = @{
        @(kAudioAggregateDeviceNameKey) : @"Babelay Tap",
        @(kAudioAggregateDeviceUIDKey) :
            [NSString stringWithFormat:@"com.babelay.tap.%@", tapUUID],
        @(kAudioAggregateDeviceIsPrivateKey) : @YES,
        @(kAudioAggregateDeviceIsStackedKey) : @NO,
        // tapautostart 는 쓰지 않는다: AudioDeviceStart 가 탭 대상이 소리를 낼 때까지 블록된다.
        @(kAudioAggregateDeviceMainSubDeviceKey) : outUID,
        @(kAudioAggregateDeviceSubDeviceListKey) : @[ @{@(kAudioSubDeviceUIDKey) : outUID} ],
        @(kAudioAggregateDeviceTapListKey) : @[ @{
            @(kAudioSubTapDriftCompensationKey) : @YES,
            @(kAudioSubTapUIDKey) : tapUUID
        } ],
    };
    AudioObjectID agg = 0;
    st = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)aggDesc, &agg);
    if (st != noErr) return (int)st;
    h->aggregate = agg;

    // 장치가 바뀌면 탭 포맷(레이트·채널)도 바뀔 수 있으므로 매번 다시 읽는다.
    AudioObjectPropertyAddress fmtAddr = {kAudioTapPropertyFormat, kAudioObjectPropertyScopeGlobal,
                                          kAudioObjectPropertyElementMain};
    AudioStreamBasicDescription asbd = {0};
    UInt32 size = sizeof(asbd);
    st = AudioObjectGetPropertyData(h->tap, &fmtAddr, 0, NULL, &size, &asbd);
    if (st != noErr) {
        close_aggregate(h);
        return (int)st;
    }
    if (!(asbd.mFormatFlags & kAudioFormatFlagIsFloat) || asbd.mBitsPerChannel != 32) {
        close_aggregate(h);
        return -2;  // float32 아닌 탭 포맷은 지원하지 않는다
    }
    if (asbd.mSampleRate <= 0 || asbd.mChannelsPerFrame == 0) {
        close_aggregate(h);
        return -2;  // 레이트/채널 0 은 리샘플러가 다룰 수 없다
    }
    h->channels = asbd.mChannelsPerFrame;
    h->rate = asbd.mSampleRate;
    h->interleaved = (asbd.mFormatFlags & kAudioFormatFlagIsNonInterleaved) == 0;

    AudioDeviceIOProcID proc = NULL;
    st = AudioDeviceCreateIOProcIDWithBlock(
        &proc, h->aggregate, NULL,
        ^(const AudioTimeStamp *now, const AudioBufferList *input, const AudioTimeStamp *inTime,
          AudioBufferList *output, const AudioTimeStamp *outTime) {
            (void)now, (void)inTime, (void)output, (void)outTime;
            if (input->mNumberBuffers == 0) return;
            if (h->interleaved) {
                for (UInt32 i = 0; i < input->mNumberBuffers; i++) {
                    const AudioBuffer *b = &input->mBuffers[i];
                    if (!b->mData) continue;
                    uint32_t ch = b->mNumberChannels ? b->mNumberChannels : h->channels;
                    if (!ch) continue;
                    uint32_t frames = b->mDataByteSize / (uint32_t)(sizeof(float) * ch);
                    if (frames) h->cb((const float *)b->mData, frames, ch, h->rate, h->user);
                }
                return;
            }
            // 논인터리브: 버퍼 하나가 채널 하나. 인터리브해서 한 번에 넘긴다.
            uint32_t ch = input->mNumberBuffers;
            if (ch != h->channels) return;  // 버퍼 수가 채널 수와 다르면 해석할 수 없다
            uint32_t frames = input->mBuffers[0].mDataByteSize / (uint32_t)sizeof(float);
            if (!frames) return;
            for (uint32_t c = 0; c < ch; c++) {
                if (input->mBuffers[c].mDataByteSize != input->mBuffers[0].mDataByteSize) return;
            }
            size_t need = (size_t)frames * ch;
            if (need > h->scratch_cap) {
                // ponytail: IO 스레드에서의 realloc. 첫 블록 이후엔 크기가 안정되어 재할당이 없다.
                // 지터가 문제되면 start 에서 최대 블록 크기로 선할당.
                float *p = realloc(h->scratch, need * sizeof(float));
                if (!p) return;
                h->scratch = p;
                h->scratch_cap = need;
            }
            for (uint32_t c = 0; c < ch; c++) {
                const float *src = (const float *)input->mBuffers[c].mData;
                if (!src) return;
                for (uint32_t f = 0; f < frames; f++) h->scratch[(size_t)f * ch + c] = src[f];
            }
            h->cb(h->scratch, frames, ch, h->rate, h->user);
        });
    if (st != noErr) {
        close_aggregate(h);
        return (int)st;
    }
    h->proc = proc;
    st = AudioDeviceStart(h->aggregate, h->proc);
    if (st != noErr) {
        close_aggregate(h);
        return (int)st;
    }
    set_out_uid(h, outUID);
    return 0;
}

int babelay_tap_probe(void) {
    if (@available(macOS 14.2, *)) {
    } else {
        return 2;  // 미지원 OS = Unknown
    }
    AudioObjectID tap = 0;
    OSStatus st = create_tap(&tap, NULL);
    if (st == noErr) {
        AudioHardwareDestroyProcessTap(tap);
        return 0;
    }
    return 1;  // 탭 생성 실패 = 권한 거부(또는 미지원)
}

int babelay_tap_start(babelay_cb cb, void *user, void **handle_out) {
    *handle_out = NULL;
    if (@available(macOS 14.2, *)) {
    } else {
        return -1;  // macOS 14.2+ 필요
    }
    tap_handle *h = calloc(1, sizeof(tap_handle));
    if (!h) return -3;
    h->cb = cb;
    h->user = user;
    CATapDescription *desc = nil;
    OSStatus st = create_tap(&h->tap, &desc);
    if (st != noErr) {
        free(h);
        return (int)st;
    }

    h->queue = (void *)CFBridgingRetain(dispatch_queue_create("com.babelay.tap",
                                                              DISPATCH_QUEUE_SERIAL));
    NSString *tapUUID = desc.UUID.UUIDString;
    int rc = open_aggregate(h, tapUUID);
    if (rc != 0) {
        babelay_tap_stop(h);
        return rc;
    }

    AudioObjectPropertyListenerBlock listener = ^(UInt32 n,
                                                 const AudioObjectPropertyAddress *addrs) {
        (void)n, (void)addrs;
        // 이 블록은 h->queue 에서 돈다(AddPropertyListenerBlock 의 큐 인자). stop 은 리스너를 먼저
        // 떼고 같은 큐에서 dispatch_sync 하므로 h 는 여기서 항상 살아 있다.
        NSString *now = nil;
        if (default_output_uid(&now) != noErr) return;
        NSString *cur = (__bridge NSString *)h->out_uid;
        if (cur && [now isEqualToString:cur]) return;  // 우리가 물고 있는 장치 그대로
        NSLog(@"babelay: default output changed %@ -> %@, rebuilding aggregate", cur, now);
        close_aggregate(h);
        int rebuild = open_aggregate(h, tapUUID);
        // 실패하면 out_uid 가 nil 로 남아 다음 변경 알림에서 다시 시도한다(폴링 없음).
        // 그동안 프레임은 오지 않는다.
        if (rebuild != 0) NSLog(@"babelay: aggregate rebuild failed (%d)", rebuild);
    };
    h->listener = (void *)CFBridgingRetain(listener);
    AudioObjectPropertyAddress defAddr = kDefaultOutputAddr;
    AudioObjectAddPropertyListenerBlock(kAudioObjectSystemObject, &defAddr,
                                        (__bridge dispatch_queue_t)h->queue,
                                        (__bridge AudioObjectPropertyListenerBlock)h->listener);

    *handle_out = h;
    return 0;
}

void babelay_tap_stop(void *handle) {
    tap_handle *h = (tap_handle *)handle;
    if (!h) return;
    if (h->listener) {
        AudioObjectPropertyAddress defAddr = kDefaultOutputAddr;
        AudioObjectRemovePropertyListenerBlock(kAudioObjectSystemObject, &defAddr,
                                               (__bridge dispatch_queue_t)h->queue,
                                               (__bridge AudioObjectPropertyListenerBlock)h->listener);
        CFBridgingRelease(h->listener);
        h->listener = NULL;
    }
    // 진행 중인 재생성이 끝난 뒤에 닫는다. 반환 시점에는 IOProc 이 없으므로 콜백도 없다.
    if (h->queue) {
        dispatch_sync((__bridge dispatch_queue_t)h->queue, ^{ close_aggregate(h); });
        CFBridgingRelease(h->queue);
        h->queue = NULL;
    } else {
        close_aggregate(h);  // start 의 초기 실패 경로(큐를 만들기 전)
    }
    if (h->tap) AudioHardwareDestroyProcessTap(h->tap);
    free(h->scratch);
    free(h);
}
