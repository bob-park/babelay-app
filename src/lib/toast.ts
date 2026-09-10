import { toast } from "react-toastify";

// 오류 알림의 단일 입구. 스토어와 페이지는 이 함수만 알고 토스트 라이브러리는 모른다.
export const showError = (e: unknown) => {
  const text = e instanceof Error ? e.message : String(e);
  // 같은 오류가 스트림처럼 쏟아질 수 있다(세션 이벤트). id 를 문구로 두면 라이브러리가 중복을 접는다.
  toast.error(text, { toastId: text });
};
