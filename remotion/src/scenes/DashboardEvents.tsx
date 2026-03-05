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

const EVENTS = [
    { time: "14:03:22", agent: "research-bot", model: "gpt-4o", tokens: "2,847", cost: "$0.08", status: "200 ✓", pii: false },
    { time: "14:03:21", agent: "code-assistant", model: "claude-4", tokens: "1,203", cost: "$0.04", status: "200 ✓", pii: false },
    { time: "14:03:19", agent: "research-bot", model: "gpt-4o", tokens: "4,102", cost: "$0.12", status: "200 ✓", pii: false },
    { time: "14:03:18", agent: "support-agent", model: "gpt-4o-m", tokens: "891", cost: "$0.01", status: "200 ✓", pii: false },
    { time: "14:03:15", agent: "research-bot", model: "gpt-4o", tokens: "3,221", cost: "$0.09", status: "⚠ PII", pii: true },
];

export const DashboardEvents: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // PII flash on last row
    const piiFlashOpacity = interpolate(
        frame % (fps * 0.8),
        [0, fps * 0.2, fps * 0.4, fps * 0.8],
        [0, 0.4, 0, 0],
        { extrapolateRight: "clamp" }
    );

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARK_BG,
                justifyContent: "center",
                alignItems: "center",
                padding: 80,
            }}
        >
            {/* Badge */}
            <div
                style={{
                    position: "absolute",
                    top: 40,
                    left: 40,
                    opacity: 0.6,
                    fontFamily: FONTS.INTER,
                    fontSize: 16,
                    fontWeight: 700,
                    color: COLORS.PURPLE,
                }}
            >
                🛡️ GOVRIX SCOUT
            </div>

            {/* Section title */}
            <div
                style={{
                    position: "absolute",
                    top: 100,
                    left: 80,
                    fontFamily: FONTS.INTER,
                    fontSize: FONT_SIZES.SUBTITLE,
                    fontWeight: 700,
                    color: COLORS.WHITE,
                    opacity: interpolate(frame, [0, fps * 0.5], [0, 1], { extrapolateRight: "clamp" }),
                }}
            >
                Event Timeline
                <span style={{ marginLeft: 16, fontSize: 18, color: COLORS.WHITE_DIM }}>
                    live
                </span>
                {/* Pulsing dot */}
                <span
                    style={{
                        marginLeft: 8,
                        display: "inline-block",
                        width: 8,
                        height: 8,
                        borderRadius: "50%",
                        backgroundColor: COLORS.GREEN,
                        opacity: interpolate(
                            frame % fps,
                            [0, fps * 0.5, fps],
                            [1, 0.3, 1]
                        ),
                    }}
                />
            </div>

            {/* Headers */}
            <div
                style={{
                    width: "100%",
                    marginTop: 60,
                    borderRadius: 16,
                    overflow: "hidden",
                    border: `1px solid ${COLORS.CARD_BORDER}`,
                    background: COLORS.CARD_BG,
                }}
            >
                <div
                    style={{
                        display: "flex",
                        padding: "16px 32px",
                        borderBottom: `1px solid ${COLORS.CARD_BORDER}`,
                        opacity: interpolate(frame, [fps * 0.2, fps * 0.6], [0, 1], {
                            extrapolateLeft: "clamp",
                            extrapolateRight: "clamp",
                        }),
                    }}
                >
                    {["Time", "Agent", "Model", "Tokens", "Cost", "Status"].map((h) => (
                        <div
                            key={h}
                            style={{
                                flex: 1,
                                fontFamily: FONTS.INTER,
                                fontSize: 13,
                                fontWeight: 600,
                                color: COLORS.WHITE_DIM,
                                textTransform: "uppercase",
                                letterSpacing: 1.5,
                            }}
                        >
                            {h}
                        </div>
                    ))}
                </div>

                {/* Event rows — slide in from top */}
                {EVENTS.map((evt, i) => {
                    const rowDelay = fps * 0.5 + i * fps * 0.35;
                    const rowEntrance = spring({
                        frame: Math.max(0, frame - rowDelay),
                        fps,
                        config: { damping: 200 },
                        durationInFrames: fps * 0.4,
                    });

                    const isPii = evt.pii;
                    const rowBg = isPii && rowEntrance > 0.9
                        ? `rgba(245, 158, 11, ${piiFlashOpacity})`
                        : "transparent";

                    return (
                        <div
                            key={i}
                            style={{
                                display: "flex",
                                padding: "18px 32px",
                                borderBottom: i < EVENTS.length - 1 ? `1px solid ${COLORS.CARD_BORDER}` : "none",
                                opacity: rowEntrance,
                                transform: `translateY(${interpolate(rowEntrance, [0, 1], [-30, 0])}px)`,
                                backgroundColor: rowBg,
                            }}
                        >
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE_DIM }}>{evt.time}</div>
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE, fontWeight: 600 }}>{evt.agent}</div>
                            <div style={{ flex: 1, fontFamily: FONTS.INTER, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE_DIM }}>{evt.model}</div>
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>{evt.tokens}</div>
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>{evt.cost}</div>
                            <div
                                style={{
                                    flex: 1,
                                    fontFamily: FONTS.INTER,
                                    fontSize: FONT_SIZES.CODE,
                                    color: isPii ? COLORS.AMBER : COLORS.GREEN,
                                    fontWeight: isPii ? 700 : 400,
                                }}
                            >
                                {evt.status}
                            </div>
                        </div>
                    );
                })}
            </div>
        </AbsoluteFill>
    );
};
