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

const LABELS = [
    { text: "🔗 SHA-256 Merkle Chain", angle: 0 },
    { text: "📋 Session Grouping", angle: 90 },
    { text: "⏱ Microsecond Timestamps", angle: 180 },
    { text: "🏷 Compliance Tags", angle: 270 },
];

export const ComplianceShield: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // Shield entrance
    const shieldEntrance = spring({
        frame,
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.8,
    });

    // Shield glow pulse
    const glowPulse = interpolate(
        frame % (fps * 2),
        [0, fps, fps * 2],
        [0.3, 0.7, 0.3],
        { extrapolateRight: "clamp" }
    );

    // Orbiting radius
    const orbitRadius = 280;

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARKER_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Radial glow */}
            <div
                style={{
                    position: "absolute",
                    width: 600,
                    height: 600,
                    borderRadius: "50%",
                    background: `radial-gradient(circle, ${COLORS.PURPLE_GLOW} 0%, transparent 70%)`,
                    opacity: glowPulse * shieldEntrance,
                }}
            />

            {/* Center shield */}
            <div
                style={{
                    opacity: shieldEntrance,
                    transform: `scale(${interpolate(shieldEntrance, [0, 1], [0.5, 1])})`,
                    zIndex: 10,
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                }}
            >
                <svg width={100} height={100} viewBox="0 0 24 24" fill="none">
                    <path
                        d="M12 2 L3 7 L3 13 C3 18 7 22 12 23 C17 22 21 18 21 13 L21 7 Z"
                        fill={COLORS.PURPLE}
                        opacity={0.2}
                        stroke={COLORS.PURPLE}
                        strokeWidth={1.5}
                    />
                    <path
                        d="M9 12 L11 14 L15 10"
                        stroke={COLORS.WHITE}
                        strokeWidth={2}
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        fill="none"
                    />
                </svg>
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: 18,
                        fontWeight: 700,
                        color: COLORS.WHITE,
                        marginTop: 12,
                        letterSpacing: 2,
                        textTransform: "uppercase",
                    }}
                >
                    Tamper-Evident
                </div>
            </div>

            {/* Orbiting labels */}
            {LABELS.map((label, i) => {
                const labelDelay = fps * 0.8 + i * fps * 0.4;
                const labelEntrance = spring({
                    frame: Math.max(0, frame - labelDelay),
                    fps,
                    config: { damping: 200 },
                    durationInFrames: fps * 0.6,
                });

                // Slow orbit rotation
                const orbitSpeed = 0.15; // degrees per frame
                const currentAngle = label.angle + frame * orbitSpeed;
                const rad = (currentAngle * Math.PI) / 180;

                const x = Math.cos(rad) * orbitRadius;
                const y = Math.sin(rad) * orbitRadius * 0.6; // Elliptical

                // Connection line
                const lineOpacity = interpolate(labelEntrance, [0, 0.5, 1], [0, 0, 0.4]);

                return (
                    <React.Fragment key={i}>
                        {/* Connection line */}
                        <svg
                            style={{
                                position: "absolute",
                                top: 0,
                                left: 0,
                                width: "100%",
                                height: "100%",
                                pointerEvents: "none",
                            }}
                        >
                            <line
                                x1="50%"
                                y1="50%"
                                x2={`calc(50% + ${x}px)`}
                                y2={`calc(50% + ${y}px)`}
                                stroke={COLORS.PURPLE}
                                strokeWidth={1}
                                opacity={lineOpacity}
                                strokeDasharray="4 4"
                            />
                        </svg>

                        {/* Label */}
                        <div
                            style={{
                                position: "absolute",
                                left: `calc(50% + ${x}px - 120px)`,
                                top: `calc(50% + ${y}px - 20px)`,
                                width: 240,
                                opacity: labelEntrance,
                                transform: `scale(${interpolate(labelEntrance, [0, 1], [0.7, 1])})`,
                                textAlign: "center",
                            }}
                        >
                            <div
                                style={{
                                    fontFamily: FONTS.INTER,
                                    fontSize: FONT_SIZES.CAPTION,
                                    fontWeight: 600,
                                    color: COLORS.WHITE,
                                    backgroundColor: COLORS.CARD_BG,
                                    border: `1px solid ${COLORS.CARD_BORDER}`,
                                    borderRadius: 12,
                                    padding: "12px 20px",
                                    backdropFilter: "blur(8px)",
                                }}
                            >
                                {label.text}
                            </div>
                        </div>
                    </React.Fragment>
                );
            })}
        </AbsoluteFill>
    );
};
