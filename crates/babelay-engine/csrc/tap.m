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
} tap_handle;

void babelay_tap_stop(void *handle);

static OSStatus default_output_uid(NSString **uid) {
    AudioObjectPropertyAddress addr = {kAudioHardwarePropertyDefaultOutputDevice,
                                       kAudioObjectPropertyScopeGlobal,
                                       kAudioObjectPropertyElementMain};
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

static OSStatus create_tap(AudioObjectID *tapOut, CATapDescription **descOut) {
    CATapDescription *desc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
    desc.name = @"Babelay";
    desc.privateTap = YES;
    desc.muteBehavior = CATapUnmuted;
    OSStatus st = AudioHardwareCreateProcessTap(desc, tapOut);
    if (st == noErr && descOut) *descOut = desc;
    return st;
}

int babelay_tap_probe(void) {
    AudioObjectID tap = 0;
    OSStatus st = create_tap(&tap, NULL);
    if (st == noErr) {
        AudioHardwareDestroyProcessTap(tap);
        return 0;
    }
    return 1;  // 탭 생성 실패 = 권한 거부(또는 미지원)
}

int babelay_tap_start(babelay_cb cb, void *user, void **handle_out) {
    tap_handle *h = calloc(1, sizeof(tap_handle));
    h->cb = cb;
    h->user = user;
    CATapDescription *desc = nil;
    OSStatus st = create_tap(&h->tap, &desc);
    if (st != noErr) {
        free(h);
        return (int)st;
    }

    NSString *outUID = nil;
    st = default_output_uid(&outUID);
    if (st != noErr) {
        AudioHardwareDestroyProcessTap(h->tap);
        free(h);
        return (int)st;
    }

    NSDictionary *aggDesc = @{
        @(kAudioAggregateDeviceNameKey) : @"Babelay Tap",
        @(kAudioAggregateDeviceUIDKey) :
            [NSString stringWithFormat:@"com.babelay.tap.%@", desc.UUID.UUIDString],
        @(kAudioAggregateDeviceIsPrivateKey) : @YES,
        @(kAudioAggregateDeviceIsStackedKey) : @NO,
        @(kAudioAggregateDeviceTapAutoStartKey) : @YES,
        @(kAudioAggregateDeviceSubDeviceListKey) : @[ @{@(kAudioSubDeviceUIDKey) : outUID} ],
        @(kAudioAggregateDeviceTapListKey) : @[ @{
            @(kAudioSubTapDriftCompensationKey) : @YES,
            @(kAudioSubTapUIDKey) : desc.UUID.UUIDString
        } ],
    };
    st = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)aggDesc, &h->aggregate);
    if (st != noErr) {
        AudioHardwareDestroyProcessTap(h->tap);
        free(h);
        return (int)st;
    }

    AudioObjectPropertyAddress fmtAddr = {kAudioTapPropertyFormat, kAudioObjectPropertyScopeGlobal,
                                          kAudioObjectPropertyElementMain};
    AudioStreamBasicDescription asbd = {0};
    UInt32 size = sizeof(asbd);
    st = AudioObjectGetPropertyData(h->tap, &fmtAddr, 0, NULL, &size, &asbd);
    if (st != noErr) {
        babelay_tap_stop(h);
        return (int)st;
    }
    h->channels = asbd.mChannelsPerFrame;
    h->rate = asbd.mSampleRate;
    h->interleaved = (asbd.mFormatFlags & kAudioFormatFlagIsNonInterleaved) == 0;

    st = AudioDeviceCreateIOProcIDWithBlock(
        &h->proc, h->aggregate, NULL,
        ^(const AudioTimeStamp *now, const AudioBufferList *input, const AudioTimeStamp *inTime,
          AudioBufferList *output, const AudioTimeStamp *outTime) {
            (void)now, (void)inTime, (void)output, (void)outTime;
            if (input->mNumberBuffers == 0) return;
            if (h->interleaved) {
                for (UInt32 i = 0; i < input->mNumberBuffers; i++) {
                    const AudioBuffer *b = &input->mBuffers[i];
                    uint32_t ch = b->mNumberChannels ? b->mNumberChannels : h->channels;
                    uint32_t frames = b->mDataByteSize / (uint32_t)(sizeof(float) * ch);
                    if (frames) h->cb((const float *)b->mData, frames, ch, h->rate, h->user);
                }
                return;
            }
            // 논인터리브: 버퍼 하나가 채널 하나. 인터리브해서 한 번에 넘긴다.
            uint32_t ch = input->mNumberBuffers;
            uint32_t frames = input->mBuffers[0].mDataByteSize / (uint32_t)sizeof(float);
            if (!frames) return;
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
        babelay_tap_stop(h);
        return (int)st;
    }
    st = AudioDeviceStart(h->aggregate, h->proc);
    if (st != noErr) {
        babelay_tap_stop(h);
        return (int)st;
    }
    *handle_out = h;
    return 0;
}

void babelay_tap_stop(void *handle) {
    tap_handle *h = (tap_handle *)handle;
    if (!h) return;
    if (h->proc) {
        AudioDeviceStop(h->aggregate, h->proc);
        AudioDeviceDestroyIOProcID(h->aggregate, h->proc);
    }
    if (h->aggregate) AudioHardwareDestroyAggregateDevice(h->aggregate);
    if (h->tap) AudioHardwareDestroyProcessTap(h->tap);
    free(h->scratch);
    free(h);
}
