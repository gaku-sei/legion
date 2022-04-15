import type { FluentBundle, FluentVariable } from "@fluent/bundle";
import type { Readable } from "svelte/store";
import { derived } from "svelte/store";

import type { BundlesStore } from "./bundles";
import type { LocaleStore } from "./locale";

export type TranslateValue = (
  id: string,
  args?: Record<string, FluentVariable> | null
) => void;

export type TranslateStore = Readable<TranslateValue>;

// TODO: Add errors support
function translate(
  locale: string,
  bundles: Map<string, FluentBundle>,
  id: string,
  args?: Record<string, FluentVariable> | null
) {
  const bundle = bundles.get(locale);

  if (!bundle) {
    return "";
  }

  const message = bundle.getMessage(id);

  return message?.value ? bundle.formatPattern(message.value, args) : "";
}

export function createTranslateStore(
  locale: LocaleStore,
  bundles: BundlesStore
): TranslateStore {
  return derived([locale, bundles], ([$locale, $bundles]) =>
    translate.bind(null, $locale, $bundles)
  );
}