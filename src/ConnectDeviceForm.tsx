import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowUpRight, LoaderCircle } from "lucide-react";
import QRReader from "./QRReader";
import {
  connectionUrl,
  readConnectionUrl,
  subnetPrefix,
  type ConnectionFields,
} from "./connection";

const storageKey = "sharex-last-device";
function rememberedDevice(): ConnectionFields {
  try {
    const value = localStorage.getItem(storageKey);
    if (value) {
      const saved = JSON.parse(value);
      connectionUrl(saved);
      return saved;
    }
  } catch {
    /* Ignore outdated or unavailable browser storage. */
  }
  return { protocol: "http", host: "", port: "6060" };
}

export default function ConnectDeviceForm({
  onOpened,
  onNotice,
}: {
  onOpened: () => void;
  onNotice: (text: string) => void;
}) {
  const [fields, setFields] = useState<ConnectionFields>(rememberedDevice);
  const [suffix, setSuffix] = useState("/");
  const [prefix, setPrefix] = useState("");
  const [networkHint, setNetworkHint] = useState(
    "Detecting this PC’s network…",
  );
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    let active = true;
    invoke<string[]>("network_addresses")
      .then((addresses) => {
        if (!active) return;
        const network = subnetPrefix(addresses[0] || "");
        setPrefix(network);
        setNetworkHint(
          network
            ? `PC network: ${network}* · Complete the other device’s IP address.`
            : "Enter the IP address shown in ShareX on the other device.",
        );
        if (network)
          setFields((current) => ({
            ...current,
            host: current.host || network,
          }));
        requestAnimationFrame(() => {
          const element = input.current;
          if (element && document.activeElement === element)
            element.setSelectionRange(
              element.value.length,
              element.value.length,
            );
        });
      })
      .catch(() => {
        if (active)
          setNetworkHint(
            "Enter the IP address shown in ShareX on the other device.",
          );
      });
    return () => {
      active = false;
    };
  }, []);
  function update(next: Partial<ConnectionFields>) {
    setFields((current) => ({ ...current, ...next }));
    setSuffix("/");
    setError("");
  }
  function readAddress(value: string) {
    const parsed = readConnectionUrl(value);
    setFields(parsed.fields);
    setSuffix(parsed.suffix);
    setError("");
  }
  let preview = "";
  try {
    preview = connectionUrl(fields, suffix);
  } catch {
    /* Show validation on submit. */
  }
  return (
    <form
      onSubmit={async (event) => {
        event.preventDefault();
        if (busy) return;
        setError("");
        setBusy(true);
        try {
          const url = connectionUrl(fields, suffix);
          await openUrl(url);
          try {
            localStorage.setItem(storageKey, JSON.stringify(fields));
          } catch {
            /* Storage is optional. */
          }
          onOpened();
        } catch (failure) {
          setError(
            failure instanceof Error ? failure.message : String(failure),
          );
        } finally {
          setBusy(false);
        }
      }}
    >
      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}
      <fieldset disabled={busy}>
        <div className="connection-fields">
          <label className="input-label" htmlFor="device-protocol">
            Protocol
            <select
              id="device-protocol"
              value={fields.protocol}
              onChange={(event) =>
                update({
                  protocol: event.target.value as ConnectionFields["protocol"],
                })
              }
            >
              <option value="http">HTTP</option>
              <option value="https">HTTPS</option>
            </select>
          </label>
          <label className="input-label" htmlFor="device-ip">
            IP address
            <input
              ref={input}
              id="device-ip"
              type="text"
              autoComplete="off"
              autoCapitalize="none"
              spellCheck={false}
              placeholder={prefix ? `${prefix}85` : "192.168.1.10"}
              value={fields.host}
              autoFocus
              required
              aria-describedby="device-network-hint"
              onChange={(event) => update({ host: event.target.value })}
              onPaste={(event) => {
                const text = event.clipboardData.getData("text").trim();
                if (/^https?:\/\//i.test(text)) {
                  event.preventDefault();
                  try {
                    readAddress(text);
                  } catch (failure) {
                    setError(
                      failure instanceof Error
                        ? failure.message
                        : String(failure),
                    );
                  }
                }
              }}
            />
          </label>
          <label className="input-label" htmlFor="device-port">
            Port
            <input
              id="device-port"
              type="number"
              min={1}
              max={65535}
              step={1}
              inputMode="numeric"
              value={fields.port}
              required
              onChange={(event) => update({ port: event.target.value })}
            />
          </label>
        </div>
        <p id="device-network-hint" className="connection-hint">
          {networkHint}
        </p>
        {preview && (
          <p className="connection-preview" aria-label="Connection URL">
            <code>{preview}</code>
          </p>
        )}
        <QRReader
          onRead={(value) => {
            try {
              readAddress(value);
              onNotice("Connection address read");
            } catch (failure) {
              setError(
                failure instanceof Error ? failure.message : String(failure),
              );
            }
          }}
        />
        <div className="modal-footer">
          <button type="submit" className="button-primary" disabled={busy}>
            {busy ? <LoaderCircle size={16} className="spin" /> : null}Open
            device
            <ArrowUpRight size={16} />
          </button>
        </div>
      </fieldset>
    </form>
  );
}
