import { AppComponent, run } from "@lgn/web-client";

import App from "@/App.svelte";

import "./assets/index.css";

const redirectUri = document.location.origin + "/";

run({
  appComponent: App as typeof AppComponent,
  auth: {
    issuerUrl:
      "https://cognito-idp.ca-central-1.amazonaws.com/ca-central-1_SkZKDimWz",
    redirectUri,
    clientId: "5m58nrjfv6kr144prif9jk62di",
    login: {
      cookies: {
        accessToken: "runtime_access_token",
        refreshToken: "runtime_refresh_token",
      },
      extraParams: {
        // eslint-disable-next-line camelcase
        identity_provider: "Azure",
      },
      scopes: [
        "aws.cognito.signin.user.admin",
        "email",
        "https://legionlabs.com/editor/allocate",
        "openid",
        "profile",
      ],
    },
  },
  rootQuerySelector: "#root",
  logLevel: "warn",
})
  // eslint-disable-next-line no-console
  .catch((error) => console.error("Application couldn't start", error));
