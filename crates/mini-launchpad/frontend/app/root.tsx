import {
  Links,
  LiveReload,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from "@remix-run/react";
import type { LinksFunction, MetaFunction } from "@remix-run/node";

import stylesheetUrl from "./styles/app.css?url";
import walletAdapterStylesUrl from "./styles/wallet-adapter-ui.css?url";

export const meta: MetaFunction = () => {
  return [
    { title: "Mini Launchpad" },
    { name: "description", content: "Минт SPL-токенов и оракул SOL/USD (Solana)" }
  ];
};

export const links: LinksFunction = () => {
  return [
    { rel: "stylesheet", href: stylesheetUrl },
    { rel: "stylesheet", href: walletAdapterStylesUrl },
  ];
};

export default function App() {
  return (
    <html lang="en">
      <head>
        <Meta />
        <Links />
      </head>
      <body>
        <Outlet />
        <ScrollRestoration />
        <Scripts />
        <LiveReload />
      </body>
    </html>
  );
}
