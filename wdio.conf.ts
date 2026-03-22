import type { Options } from '@wdio/types'

export const config: Options.Testrunner = {
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json',
      transpileOnly: true,
    },
  },

  specs: ['./e2e/**/*.test.ts'],
  maxInstances: 1,

  capabilities: [
    {
      // @ts-expect-error tauri-specific capability
      'tauri:options': {
        application: './src-tauri/target/debug/fileflow',
      },
      browserName: '',
    },
  ],

  services: [
    [
      'tauri',
      {
        tauriDriverPath: 'tauri-driver',
      },
    ],
  ],

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
}
