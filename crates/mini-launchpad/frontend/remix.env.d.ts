/// <reference types="@remix-run/node" />
/// <reference types="@remix-run/react" />
/// <reference types="@remix-run/serve" />

declare module "*.css?url" {
  const href: string;
  export default href;
}
