export type ConnectionProtocol = "http" | "https";
export interface ConnectionFields {
  protocol: ConnectionProtocol;
  host: string;
  port: string;
}

function normalizedIp(value: string): string {
  const host = value.trim();
  if (/^(\d{1,3}\.){3}\d{1,3}$/.test(host)) {
    const parts = host.split(".").map(Number);
    if (parts.every((part) => part >= 0 && part <= 255)) return parts.join(".");
  } else if (host.includes(":")) {
    const address = host.replace(/^\[|\]$/g, "");
    try {
      return new URL(`http://[${address}]/`).hostname;
    } catch {
      /* Invalid IPv6 address. */
    }
  }
  throw new Error(
    "Enter the other device’s complete IP address, such as 192.168.1.10.",
  );
}

export function subnetPrefix(address: string): string {
  if (!/^(\d{1,3}\.){3}\d{1,3}$/.test(address)) return "";
  try {
    return normalizedIp(address).split(".").slice(0, 3).join(".") + ".";
  } catch {
    return "";
  }
}

export function connectionUrl(fields: ConnectionFields, suffix = "/"): string {
  if (fields.protocol !== "http" && fields.protocol !== "https") {
    throw new Error("Choose HTTP or HTTPS.");
  }
  const host = normalizedIp(fields.host);
  const port = fields.port.trim();
  if (!/^\d{1,5}$/.test(port) || Number(port) < 1 || Number(port) > 65535) {
    throw new Error("Enter a port between 1 and 65535.");
  }
  const base = new URL(`${fields.protocol}://${host}:${Number(port)}/`);
  // QR paths stay on the same device. Never accept an origin-relative URL here.
  base.pathname = "/";
  if (suffix && suffix !== "/") {
    const target = new URL(suffix, base);
    if (target.origin !== base.origin)
      throw new Error("Connection path must stay on the same device.");
    return target.href;
  }
  return base.href;
}

export function readConnectionUrl(value: string): {
  fields: ConnectionFields;
  suffix: string;
} {
  let url: URL;
  try {
    url = new URL(value.trim());
  } catch {
    throw new Error("Use an HTTP or HTTPS connection address from ShareX.");
  }
  if (
    !["http:", "https:"].includes(url.protocol) ||
    url.username ||
    url.password
  ) {
    throw new Error(
      "Use an HTTP or HTTPS connection address without a username or password.",
    );
  }
  const fields: ConnectionFields = {
    protocol: url.protocol === "https:" ? "https" : "http",
    host: normalizedIp(url.hostname).replace(/^\[|\]$/g, ""),
    port: url.port || (url.protocol === "https:" ? "443" : "80"),
  };
  connectionUrl(fields);
  return { fields, suffix: url.pathname + url.search + url.hash };
}
