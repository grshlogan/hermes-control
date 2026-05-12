import { describe, expect, it } from 'vitest';
import {
  DEFAULT_LANGUAGE,
  createTranslator,
  languageOptions,
  normalizeLanguage,
} from './i18n';
import {
  buildHermesActionOptions,
  buildModelActionOptions,
  buildWslActionOptions,
} from './viewModel';

describe('Phase8 GUI i18n', () => {
  it('defaults the GUI language to Simplified Chinese', () => {
    expect(DEFAULT_LANGUAGE).toBe('zh-CN');
    expect(createTranslator(DEFAULT_LANGUAGE)('nav.settings')).toBe('设置');
    expect(createTranslator(DEFAULT_LANGUAGE)('topbar.title')).toBe('运维控制台');
  });

  it('keeps English available as an explicit operator choice', () => {
    expect(languageOptions).toEqual([
      { id: 'zh-CN', label: '简体中文' },
      { id: 'en-US', label: 'English' },
    ]);
    expect(createTranslator('en-US')('settings.language')).toBe('Language');
  });

  it('normalizes unsupported language values back to Chinese', () => {
    expect(normalizeLanguage('en-US')).toBe('en-US');
    expect(normalizeLanguage('fr-FR')).toBe('zh-CN');
    expect(normalizeLanguage(null)).toBe('zh-CN');
  });

  it('localizes common action option labels without changing typed action ids', () => {
    const t = createTranslator('zh-CN');

    expect(buildModelActionOptions(t)[0]).toEqual({
      id: 'Install',
      label: '安装',
      riskHint: '普通',
    });
    expect(buildWslActionOptions(t)[3]).toEqual({
      id: 'ShutdownAll',
      label: '全部关闭',
      riskHint: '破坏性',
    });
    expect(buildHermesActionOptions(t)[3]).toEqual({
      id: 'Kill',
      label: '强杀',
      riskHint: '破坏性',
    });
  });
});
