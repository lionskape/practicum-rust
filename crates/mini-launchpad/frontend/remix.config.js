/** @type {import('@remix-run/dev').AppConfig} */
export default {
  ignoredRouteFiles: ["**/*.css.map", "**/*.test.*"],
  serverModuleFormat: "esm",
  tailwind: false,
  browserNodeBuiltinsPolyfill: {
    modules: {
      stream: true,
      string_decoder: true,
      assert: true,
      buffer: true,
      events: true,
    },
  },
};
