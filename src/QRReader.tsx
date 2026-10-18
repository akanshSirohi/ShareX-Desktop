import { useEffect, useRef, useState } from "react";
import jsQR from "jsqr";
import { Camera, ImagePlus, Square } from "lucide-react";

export default function QRReader({
  onRead,
}: {
  onRead: (value: string) => void;
}) {
  const [camera, setCamera] = useState(false),
    [error, setError] = useState("");
  const video = useRef<HTMLVideoElement>(null);
  function decode(source: CanvasImageSource, width: number, height: number) {
    const scale = Math.min(1, 1200 / Math.max(width, height));
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(width * scale);
    canvas.height = Math.round(height * scale);
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) return null;
    context.drawImage(source, 0, 0, canvas.width, canvas.height);
    const image = context.getImageData(0, 0, canvas.width, canvas.height);
    return jsQR(image.data, image.width, image.height)?.data;
  }
  useEffect(() => {
    if (!camera) return;
    if (!navigator.mediaDevices?.getUserMedia) {
      setError(
        "Camera unavailable. Use a QR image or enter the connection address.",
      );
      setCamera(false);
      return;
    }
    let stream: MediaStream | undefined;
    let timer: ReturnType<typeof setInterval> | undefined;
    let cancelled = false;
    navigator.mediaDevices
      ?.getUserMedia({ video: { facingMode: "environment" }, audio: false })
      .then((s) => {
        if (cancelled) {
          s.getTracks().forEach((t) => t.stop());
          return;
        }
        stream = s;
        if (video.current) {
          video.current.srcObject = s;
          video.current.play();
        }
        timer = setInterval(() => {
          const v = video.current;
          if (!v?.videoWidth) return;
          const value = decode(v, v.videoWidth, v.videoHeight);
          if (value) {
            onRead(value);
            setCamera(false);
          }
        }, 350);
      })
      .catch((e) => {
        setError(String(e));
        setCamera(false);
      });
    return () => {
      cancelled = true;
      if (timer) clearInterval(timer);
      stream?.getTracks().forEach((t) => t.stop());
    };
  }, [camera]);
  return (
    <div className="qr-reader">
      <div className="qr-reader-actions">
        <button
          type="button"
          className="button"
          onClick={() => {
            setError("");
            setCamera(!camera);
          }}
        >
          {camera ? <Square size={14} /> : <Camera size={15} />}
          {camera ? "Stop camera" : "Scan QR code"}
        </button>
        <label className="button">
          <ImagePlus size={15} />
          Read QR image
          <input
            type="file"
            accept="image/*"
            className="sr-only"
            onChange={async (e) => {
              const file = e.target.files?.[0];
              if (!file) return;
              try {
                const bitmap = await createImageBitmap(file);
                const value = decode(bitmap, bitmap.width, bitmap.height);
                bitmap.close();
                if (!value) throw new Error("No QR code found in this image.");
                onRead(value);
                setError("");
              } catch (e) {
                setError(String(e));
              }
            }}
          />
        </label>
      </div>
      {camera && (
        <video className="qr-video" ref={video} autoPlay muted playsInline />
      )}
      {error && (
        <p className="danger-text" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
