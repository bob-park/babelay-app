import { toast } from "react-toastify";

// 오류 알림의 단일 입구. 스토어와 페이지는 이 함수만 알고 토스트 라이브러리는 모른다.
export const showError = (e: unknown) => {
  toast.error(e instanceof Error ? e.message : String(e));
};
