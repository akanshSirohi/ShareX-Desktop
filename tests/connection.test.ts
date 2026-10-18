import { test } from "node:test";
import assert from "node:assert/strict";
import {
  connectionUrl,
  readConnectionUrl,
  subnetPrefix,
} from "../src/connection.ts";

test("Android LAN addresses preserve protocol, host and custom port", () => {
  assert.equal(
    connectionUrl({ protocol: "http", host: "192.0.2.20", port: "6060" }),
    "http://192.0.2.20:6060/",
  );
  assert.equal(
    connectionUrl({ protocol: "https", host: "192.0.2.20", port: "7443" }),
    "https://192.0.2.20:7443/",
  );
});
test("network prefill leaves destination octet for the user", () => {
  assert.equal(subnetPrefix("192.0.2.10"), "192.0.2.");
  assert.equal(subnetPrefix("not-an-address"), "");
  assert.throws(
    () =>
      connectionUrl({ protocol: "http", host: "192.0.2.", port: "6060" }),
    /complete IP/,
  );
});
test("rejects invalid addresses and ports instead of URL shorthand or octal interpretation", () => {
  for (const host of [
    "192.0.2.999",
    "192.0.2",
    "192.0.2.20/path",
    "192.0.2.20@evil.example",
    "file:///C:/",
  ]) {
    assert.throws(() =>
      connectionUrl({ protocol: "http", host, port: "6060" }),
    );
  }
  for (const port of ["0", "65536", "60.60", "1e3", "", "6060/path"]) {
    assert.throws(() =>
      connectionUrl({ protocol: "http", host: "192.0.2.20", port }),
    );
  }
  assert.equal(
    connectionUrl({ protocol: "http", host: "192.000.002.020", port: "6060" }),
    "http://192.0.2.20:6060/",
  );
});
test("QR and pasted URLs populate all fields, including default ports and paths", () => {
  const parsed = readConnectionUrl(
    "https://192.0.2.20:7443/#location=Photos",
  );
  assert.deepEqual(parsed.fields, {
    protocol: "https",
    host: "192.0.2.20",
    port: "7443",
  });
  assert.equal(
    connectionUrl(parsed.fields, parsed.suffix),
    "https://192.0.2.20:7443/#location=Photos",
  );
  assert.equal(readConnectionUrl("http://192.0.2.20/").fields.port, "80");
  assert.equal(readConnectionUrl("https://192.0.2.20/").fields.port, "443");
});
test("IPv6 device URLs round trip without losing address brackets", () => {
  const parsed = readConnectionUrl("http://[fd12::85]:6060/");
  assert.equal(parsed.fields.host, "fd12::85");
  assert.equal(connectionUrl(parsed.fields), "http://[fd12::85]:6060/");
});
test("rejects alternate schemes, credentials and paths that change the target device", () => {
  for (const url of [
    "javascript:alert(1)",
    "file:///C:/",
    "ftp://192.0.2.20/",
    "http://user:pass@192.0.2.20/",
  ]) {
    assert.throws(() => readConnectionUrl(url));
  }
  assert.throws(
    () =>
      connectionUrl(
        { protocol: "http", host: "192.0.2.20", port: "6060" },
        "//192.0.2.1/",
      ),
    /same device/,
  );
});
