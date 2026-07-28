/** Jest configuration for frontend tests */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "jsdom",
  globals: {
    "ts-jest": {
      tsconfig: "tsconfig.test.json",
    },
  },
  testMatch: [
    "**/__tests__/**/*.test.{ts,tsx}",
    "**/?(*.)+(spec|test).{ts,tsx}",
  ],
  moduleNameMapper: {
    "^@/(.*)$": "<rootDir>/$1",
  },
  setupFilesAfterEnv: ["<rootDir>/setupTests.ts"],
  collectCoverageFrom: [
    "lib/**/*.ts",
    "hooks/**/*.ts",
    "components/**/*.tsx",
    "services/**/*.ts",
    "utils/**/*.ts",
    "!lib/**/mock-data.ts",
    "!**/*.d.ts",
  ],
  coverageDirectory: "<rootDir>/coverage",
};
