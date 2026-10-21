// @ts-check
import withNuxt from './.nuxt/eslint.config.mjs'
import oxlint from 'eslint-plugin-oxlint'
import prettier from 'eslint-config-prettier/flat'

// oxfmt formats and oxlint lints; eslint keeps the Vue template rules oxlint lacks.
export default withNuxt(...oxlint.buildFromOxlintConfigFile('.oxlintrc.json'), prettier, {
  files: ['**/*.vue'],
  rules: { '@typescript-eslint/no-unused-vars': 'error' }
})
