import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 90_000,
  workers: 1,
  use: {
    baseURL: 'http://127.0.0.1:8082',
    launchOptions: process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {},
  },
  webServer: [
    {
      command: 'cargo run -p eins_server --locked',
      url: 'http://127.0.0.1:8080/health',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
    {
      command: 'trunk serve --release --locked',
      url: 'http://127.0.0.1:8082',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
  ],
});
