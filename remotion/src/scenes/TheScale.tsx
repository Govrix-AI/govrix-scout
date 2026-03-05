import React from "react";
import {
    AbsoluteFill,
    interpolate,
    spring,
    useCurrentFrame,
    useVideoConfig,
} from "remotion";
import { COLORS, FONT_SIZES } from "../utils/colors";
import { FONTS } from "../utils/fonts";

// Generate city positions on a "world map"
const NODES = [
    { x: 280, y: 340, label: "NYC" },
    { x: 380, y: 280, label: "London" },
    { x: 580, y: 300, label: "Berlin" },
    { x: 900, y: 400, label: "Mumbai" },
    { x: 1100, y: 320, label: "Singapore" },
    { x: 1300, y: 350, label: "Tokyo" },
    { x: 1500, y: 450, label: "Sydney" },
    { x: 350, y: 500, label: "São Paulo" },
    { x: 700, y: 260, label: "Moscow" },
    { x: 450, y: 450, label: "Lagos" },
];

const STATS = [
    { label: "Agents Tracked", value: 12847 },
    { label: "Events Logged", value: 4200000, suffix: "M", divisor: 1000000 },
    { label: "Cost Attributed", value: 847000, prefix: "$", suffix: "K", divisor: 1000 },
];

export const TheScale: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARKER_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* "Globe" wireframe background */}
            <div
                style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    backgroundImage: `
            radial-gradient(ellipse 1600px 700px at 50% 45%, rgba(139,92,246,0.05) 0%, transparent 100%),
            linear-gradient(rgba(255,255,255,0.015) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255,255,255,0.015) 1px, transparent 1px)
          `,
                    backgroundSize: "100% 100%, 80px 80px, 80px 80px",
                }}
            />

            {/* Connection lines between nodes */}
            <svg
                style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: "100%",
                }}
            >
                {NODES.slice(0, -1).map((node, i) => {
                    const next = NODES[i + 1];
                    const lineDelay = fps * 0.5 + i * fps * 0.15;
                    const lineProgress = interpolate(
                        frame,
                        [lineDelay, lineDelay + fps * 0.4],
                        [0, 1],
                        { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
                    );

                    return (
                        <line
                            key={`line-${i}`}
                            x1={node.x}
                            y1={node.y}
                            x2={node.x + (next.x - node.x) * lineProgress}
                            y2={node.y + (next.y - node.y) * lineProgress}
                            stroke={COLORS.PURPLE}
                            strokeWidth={1}
                            opacity={0.3 * lineProgress}
                        />
                    );
                })}
            </svg>

            {/* City nodes */}
            {NODES.map((node, i) => {
                const nodeDelay = fps * 0.3 + i * fps * 0.2;
                const nodeEntrance = spring({
                    frame: Math.max(0, frame - nodeDelay),
                    fps,
                    config: { damping: 200 },
                    durationInFrames: fps * 0.4,
                });

                // Pulsing glow
                const pulsePhase = ((frame + i * 10) % (fps * 1.5)) / (fps * 1.5);
                const pulseSize = interpolate(pulsePhase, [0, 0.5, 1], [6, 14, 6]);

                return (
                    <div
                        key={i}
                        style={{
                            position: "absolute",
                            left: node.x - 6,
                            top: node.y - 6,
                            opacity: nodeEntrance,
                            transform: `scale(${interpolate(nodeEntrance, [0, 1], [0, 1])})`,
                        }}
                    >
                        {/* Glow ring */}
                        <div
                            style={{
                                position: "absolute",
                                width: pulseSize * 2,
                                height: pulseSize * 2,
                                borderRadius: "50%",
                                backgroundColor: COLORS.PURPLE_GLOW,
                                top: 6 - pulseSize,
                                left: 6 - pulseSize,
                            }}
                        />
                        {/* Core dot */}
                        <div
                            style={{
                                width: 12,
                                height: 12,
                                borderRadius: "50%",
                                backgroundColor: COLORS.PURPLE,
                                boxShadow: `0 0 10px ${COLORS.PURPLE}`,
                            }}
                        />
                    </div>
                );
            })}

            {/* Bottom stats bar */}
            <div
                style={{
                    position: "absolute",
                    bottom: 80,
                    display: "flex",
                    gap: 100,
                    justifyContent: "center",
                    width: "100%",
                }}
            >
                {STATS.map((stat, i) => {
                    const statDelay = fps * 1 + i * fps * 0.3;
                    const statEntrance = spring({
                        frame: Math.max(0, frame - statDelay),
                        fps,
                        config: { damping: 200 },
                        durationInFrames: fps * 0.6,
                    });

                    const displayValue = stat.divisor
                        ? (interpolate(statEntrance, [0, 1], [0, stat.value / stat.divisor])).toFixed(stat.value > 1000000 ? 1 : 0)
                        : Math.round(interpolate(statEntrance, [0, 1], [0, stat.value])).toLocaleString();

                    return (
                        <div
                            key={i}
                            style={{
                                textAlign: "center",
                                opacity: statEntrance,
                            }}
                        >
                            <div
                                style={{
                                    fontFamily: FONTS.MONO,
                                    fontSize: 42,
                                    fontWeight: 700,
                                    color: COLORS.WHITE,
                                }}
                            >
                                {stat.prefix || ""}{displayValue}{stat.suffix || ""}
                            </div>
                            <div
                                style={{
                                    fontFamily: FONTS.INTER,
                                    fontSize: 16,
                                    color: COLORS.WHITE_DIM,
                                    marginTop: 8,
                                    textTransform: "uppercase",
                                    letterSpacing: 1.5,
                                }}
                            >
                                {stat.label}
                            </div>
                        </div>
                    );
                })}
            </div>
        </AbsoluteFill>
    );
};
