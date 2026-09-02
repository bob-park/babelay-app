import { create } from "zustand";
import type { ModelStatus } from "./types";

interface ModelsStore { models: ModelStatus[] }
export const useModels = create<ModelsStore>(() => ({ models: [] }));
