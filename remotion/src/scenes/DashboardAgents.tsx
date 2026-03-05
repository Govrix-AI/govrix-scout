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

const AGENTS = [
    { name: "research-bot", framework: "LangChain", status: "active", requests: "12,847", cost: "$142.30", color: COLORS.GREEN },
    { name: "code-assistant", framework: "CrewAI", status: "active", requests: "8,231", cost: "$89.44", color: COLORS.GREEN },
    { name: "support-agent", framework: "AutoGen", status: "idle", requests: "342", cost: "$3.71", color: COLORS.AMBER },
    { name: "unknown-10.0.3.7", framework: "—", status: "unknown", requests: "17", cost: "$0.89", color: COLORS.RED },
];

const HEADERS = ["Agent", "Framework", "Status", "Requests", "Cost (24h)"];

export const DashboardAgents: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARK_BG,
                justifyContent: "center",
                alignItems: "center",
                padding: 80,
            }}
        >
            {/* Small logo badge top-left */}
            <div
                style={{
                    position: "absolute",
                    top: 40,
                    left: 40,
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    opacity: 0.6,
                }}
            >
                <span style={{ fontFamily: FONTS.INTER, fontSize: 16, fontWeight: 700, color: COLORS.PURPLE }}>
                    🛡️ GOVRIX SCOUT
                </span>
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
                }}
            >
                <span style={{ opacity: interpolate(frame, [0, fps * 0.5], [0, 1], { extrapolateRight: "clamp" }) }}>
                    Agent Inventory
                </span>
                <span
                    style={{
                        marginLeft: 16,
                        fontSize: 18,
                        color: COLORS.GREEN,
                        opacity: interpolate(frame, [fps * 0.3, fps * 0.8], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }),
                    }}
                >
                    3 active
                </span>
            </div>

            {/* Table */}
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
                {/* Header row */}
                <div
                    style={{
                        display: "flex",
                        padding: "20px 32px",
                        borderBottom: `1px solid ${COLORS.CARD_BORDER}`,
                        opacity: interpolate(frame, [fps * 0.3, fps * 0.7], [0, 1], {
                            extrapolateLeft: "clamp",
                            extrapolateRight: "clamp",
                        }),
                    }}
                >
                    {HEADERS.map((h) => (
                        <div
                            key={h}
                            style={{
                                flex: 1,
                                fontFamily: FONTS.INTER,
                                fontSize: 14,
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

                {/* Agent rows */}
                {AGENTS.map((agent, i) => {
                    const rowDelay = fps * 0.8 + i * fps * 0.4;
                    const rowEntrance = spring({
                        frame: Math.max(0, frame - rowDelay),
                        fps,
                        config: { damping: 200 },
                        durationInFrames: fps * 0.5,
                    });

                    return (
                        <div
                            key={i}
                            style={{
                                display: "flex",
                                padding: "22px 32px",
                                borderBottom: i < AGENTS.length - 1 ? `1px solid ${COLORS.CARD_BORDER}` : "none",
                                opacity: rowEntrance,
                                transform: `translateY(${interpolate(rowEntrance, [0, 1], [20, 0])}px)`,
                            }}
                        >
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE, fontWeight: 600 }}>
                                {agent.name}
                            </div>
                            <div style={{ flex: 1, fontFamily: FONTS.INTER, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE_DIM }}>
                                {agent.framework}
                            </div>
                            <div style={{ flex: 1, fontFamily: FONTS.INTER, fontSize: FONT_SIZES.CODE, color: agent.color, display: "flex", alignItems: "center", gap: 8 }}>
                                <span style={{ width: 8, height: 8, borderRadius: "50%", backgroundColor: agent.color, display: "inline-block" }} />
                                {agent.status}
                            </div>
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>
                                {agent.requests}
                            </div>
                            <div style={{ flex: 1, fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>
                                {agent.cost}
                            </div>
                        </div>
                    );
                })}
            </div>
        </AbsoluteFill>
    );
};
