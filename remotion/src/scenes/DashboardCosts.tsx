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

const BARS_BY_AGENT = [
    { label: "research-bot", value: 412, maxWidth: 500, color: COLORS.PURPLE },
    { label: "code-assistant", value: 267, maxWidth: 324, color: "#6366F1" },
    { label: "support-agent", value: 89, maxWidth: 108, color: "#818CF8" },
    { label: "data-pipeline", value: 79, maxWidth: 96, color: "#A78BFA" },
];

const BARS_BY_MODEL = [
    { label: "gpt-4o", value: 523, maxWidth: 500, color: "#22D3EE" },
    { label: "claude-4", value: 198, maxWidth: 189, color: "#06B6D4" },
    { label: "gpt-4o-mini", value: 89, maxWidth: 85, color: "#67E8F9" },
    { label: "gpt-3.5", value: 37, maxWidth: 35, color: "#A5F3FC" },
];

export const DashboardCosts: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    const totalReveal = spring({
        frame,
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.8,
    });

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
            <div style={{ position: "absolute", top: 40, left: 40, opacity: 0.6, fontFamily: FONTS.INTER, fontSize: 16, fontWeight: 700, color: COLORS.PURPLE }}>
                🛡️ GOVRIX SCOUT
            </div>

            {/* Title */}
            <div
                style={{
                    position: "absolute",
                    top: 100,
                    left: 80,
                    fontFamily: FONTS.INTER,
                    fontSize: FONT_SIZES.SUBTITLE,
                    fontWeight: 700,
                    color: COLORS.WHITE,
                    opacity: totalReveal,
                }}
            >
                Cost Breakdown
                <span style={{ marginLeft: 16, fontSize: 20, color: COLORS.WHITE_DIM }}>
                    Last 7 days: <span style={{ color: COLORS.WHITE, fontWeight: 700 }}>$847.21</span>
                </span>
            </div>

            {/* Two columns */}
            <div
                style={{
                    display: "flex",
                    gap: 60,
                    width: "100%",
                    marginTop: 80,
                }}
            >
                {/* By Agent */}
                <div style={{ flex: 1 }}>
                    <div style={{ fontFamily: FONTS.INTER, fontSize: 16, fontWeight: 600, color: COLORS.WHITE_DIM, marginBottom: 24, textTransform: "uppercase", letterSpacing: 1.5 }}>
                        By Agent
                    </div>
                    {BARS_BY_AGENT.map((bar, i) => {
                        const barDelay = fps * 0.5 + i * fps * 0.25;
                        const barProgress = spring({
                            frame: Math.max(0, frame - barDelay),
                            fps,
                            config: { damping: 200 },
                            durationInFrames: fps * 0.6,
                        });

                        const animatedCost = Math.round(interpolate(barProgress, [0, 1], [0, bar.value]));

                        return (
                            <div key={i} style={{ marginBottom: 20 }}>
                                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                                    <span style={{ fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>{bar.label}</span>
                                    <span style={{ fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>${animatedCost}</span>
                                </div>
                                <div style={{ height: 12, backgroundColor: "rgba(255,255,255,0.06)", borderRadius: 6 }}>
                                    <div
                                        style={{
                                            height: "100%",
                                            width: interpolate(barProgress, [0, 1], [0, bar.maxWidth]),
                                            backgroundColor: bar.color,
                                            borderRadius: 6,
                                            boxShadow: `0 0 12px ${bar.color}40`,
                                        }}
                                    />
                                </div>
                            </div>
                        );
                    })}
                </div>

                {/* By Model */}
                <div style={{ flex: 1 }}>
                    <div style={{ fontFamily: FONTS.INTER, fontSize: 16, fontWeight: 600, color: COLORS.WHITE_DIM, marginBottom: 24, textTransform: "uppercase", letterSpacing: 1.5 }}>
                        By Model
                    </div>
                    {BARS_BY_MODEL.map((bar, i) => {
                        const barDelay = fps * 0.7 + i * fps * 0.25;
                        const barProgress = spring({
                            frame: Math.max(0, frame - barDelay),
                            fps,
                            config: { damping: 200 },
                            durationInFrames: fps * 0.6,
                        });

                        const animatedCost = Math.round(interpolate(barProgress, [0, 1], [0, bar.value]));

                        return (
                            <div key={i} style={{ marginBottom: 20 }}>
                                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                                    <span style={{ fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>{bar.label}</span>
                                    <span style={{ fontFamily: FONTS.MONO, fontSize: FONT_SIZES.CODE, color: COLORS.WHITE }}>${animatedCost}</span>
                                </div>
                                <div style={{ height: 12, backgroundColor: "rgba(255,255,255,0.06)", borderRadius: 6 }}>
                                    <div
                                        style={{
                                            height: "100%",
                                            width: interpolate(barProgress, [0, 1], [0, bar.maxWidth]),
                                            backgroundColor: bar.color,
                                            borderRadius: 6,
                                            boxShadow: `0 0 12px ${bar.color}40`,
                                        }}
                                    />
                                </div>
                            </div>
                        );
                    })}
                </div>
            </div>
        </AbsoluteFill>
    );
};
