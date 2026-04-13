const tseslint = require('typescript-eslint')
const eslintConfigPrettier = require('eslint-config-prettier')

module.exports = tseslint.config(
  {
    ignores: ['**/dist/', '**/templates/', 'eslint.config.js']
  },
  ...tseslint.configs.recommended,
  eslintConfigPrettier,
  {
    languageOptions: {
      parserOptions: {
        project: 'tsconfig.json'
      }
    }
  }
)
