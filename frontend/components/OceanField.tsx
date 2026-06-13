"use client";

import { useEffect, useRef } from "react";

type Point = {
  x: number;
  y: number;
  radius: number;
  drift: number;
  phase: number;
};

export default function OceanField() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const context = canvas.getContext("2d");
    if (!context) return;

    let frame = 0;
    let width = 0;
    let height = 0;
    let points: Point[] = [];
    let animationFrame = 0;
    const reducedMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;

    const resize = () => {
      const bounds = canvas.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      width = bounds.width;
      height = bounds.height;
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);

      const count = Math.max(45, Math.floor((width * height) / 9000));
      points = Array.from({ length: count }, (_, index) => ({
        x: ((index * 89) % 101) / 101,
        y: ((index * 53) % 97) / 97,
        radius: 0.6 + ((index * 7) % 13) / 10,
        drift: 0.002 + ((index * 11) % 9) / 2500,
        phase: index * 0.73,
      }));
    };

    const drawRoute = (
      fromX: number,
      fromY: number,
      toX: number,
      toY: number,
      progress: number,
    ) => {
      const controlX = width * 0.5;
      const controlY = Math.min(fromY, toY) - height * 0.24;

      context.save();
      context.beginPath();
      context.moveTo(fromX, fromY);
      context.quadraticCurveTo(controlX, controlY, toX, toY);
      context.strokeStyle = "rgba(116, 255, 214, 0.13)";
      context.lineWidth = 1;
      context.stroke();

      context.beginPath();
      context.moveTo(fromX, fromY);
      context.quadraticCurveTo(controlX, controlY, toX, toY);
      context.strokeStyle = "rgba(116, 255, 214, 0.82)";
      context.lineWidth = 1.4;
      context.setLineDash([8, 14]);
      context.lineDashOffset = -progress * 42;
      context.shadowColor = "rgba(91, 255, 208, 0.8)";
      context.shadowBlur = 10;
      context.stroke();
      context.restore();

      const t = (progress * 0.08) % 1;
      const inverse = 1 - t;
      const x =
        inverse * inverse * fromX +
        2 * inverse * t * controlX +
        t * t * toX;
      const y =
        inverse * inverse * fromY +
        2 * inverse * t * controlY +
        t * t * toY;

      context.save();
      context.beginPath();
      context.arc(x, y, 3.5, 0, Math.PI * 2);
      context.fillStyle = "#ecfff9";
      context.shadowColor = "#66ffd1";
      context.shadowBlur = 16;
      context.fill();
      context.restore();
    };

    const draw = () => {
      context.clearRect(0, 0, width, height);
      const time = reducedMotion ? 12 : frame * 0.012;

      const glow = context.createRadialGradient(
        width * 0.55,
        height * 0.45,
        0,
        width * 0.55,
        height * 0.45,
        width * 0.55,
      );
      glow.addColorStop(0, "rgba(40, 179, 146, 0.11)");
      glow.addColorStop(0.42, "rgba(25, 83, 105, 0.07)");
      glow.addColorStop(1, "rgba(4, 10, 15, 0)");
      context.fillStyle = glow;
      context.fillRect(0, 0, width, height);

      for (const point of points) {
        const px =
          point.x * width +
          Math.sin(time * point.drift * 120 + point.phase) * 6;
        const py =
          point.y * height +
          Math.cos(time * point.drift * 90 + point.phase) * 4;
        context.beginPath();
        context.arc(px, py, point.radius, 0, Math.PI * 2);
        context.fillStyle = `rgba(145, 202, 203, ${
          0.12 + Math.sin(time + point.phase) * 0.05
        })`;
        context.fill();
      }

      for (let row = 1; row < 6; row += 1) {
        context.beginPath();
        for (let x = 0; x <= width; x += 12) {
          const y =
            (height / 6) * row +
            Math.sin(x * 0.012 + time + row) * (2 + row * 0.4);
          if (x === 0) context.moveTo(x, y);
          else context.lineTo(x, y);
        }
        context.strokeStyle = "rgba(104, 178, 190, 0.055)";
        context.lineWidth = 1;
        context.stroke();
      }

      drawRoute(
        width * 0.17,
        height * 0.63,
        width * 0.83,
        height * 0.36,
        time,
      );

      if (!reducedMotion) {
        frame += 1;
        animationFrame = requestAnimationFrame(draw);
      }
    };

    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();
    draw();

    return () => {
      observer.disconnect();
      cancelAnimationFrame(animationFrame);
    };
  }, []);

  return <canvas ref={canvasRef} className="ocean-field" aria-hidden="true" />;
}
