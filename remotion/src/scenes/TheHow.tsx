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

const CODE_LINE = 'export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1';

const FLOW_NODES = [
    { label: "Your Agent", icon: "🤖", x: 200, y: 500 },
    { label: "Govrix Proxy\n:4000", icon: "🛡️", x: 700, y: 500 },
    { label: "OpenAI / Anthropic", icon: "☁️", x: 1200, y: 500 },
];

const FLOW_DOWN_NODES = [
    { label: "Log + Attribute\n+ Scan", icon: "📋", x: 700, y: 680 },
    { label: "TimescaleDB\n+ Dashboard", icon: "💾", x: 700, y: 850 },
];

export const TheHow: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // Phase 1: Code editor (0 - 3s)
    const codeOpacity = interpolate(frame, [0, fps * 0.5], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
    });

    const codeHighlight = interpolate(
        frame,
        [fps * 1, fps * 1.5],
        [0, 1],
        { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );

    // Phase 2: Code slides up, flow diagram appears (3s+)
    const transitionProgress = spring({
        frame: Math.max(0, frame - fps * 3.5),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.8,
    });

    const codeY = interpolate(transitionProgress, [0, 1], [0, -280]);

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARK_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Code editor */}
            <div
                style={{
                    position: "absolute",
                    top: 200,
                    transform: `translateY(${codeY}px)`,
                    opacity: codeOpacity,
                    width: 1400,
                }}
            >
                {/* Editor chrome */}
                <div
                    style={{
                        backgroundColor: "#1e1e2e",
                        borderRadius: "12px 12px 0 0",
                        padding: "12px 20px",
                        display: "flex",
                        gap: 8,
                    }}
                >
                    <div style={{ width: 12, height: 12, borderRadius: "50%", backgroundColor: "#FF5F57" }} />
                    <div style={{ width: 12, height: 12, borderRadius: "50%", backgroundColor: "#FEBC2E" }} />
                    <div style={{ width: 12, height: 12, borderRadius: "50%", backgroundColor: "#28C840" }} />
                    <span style={{ marginLeft: 12, fontFamily: FONTS.MONO, fontSize: 13, color: COLORS.WHITE_DIM }}>terminal</span>
                </div>

                {/* Code area */}
                <div
                    style={{
                        backgroundColor: COLORS.CODE_BG,
                        borderRadius: "0 0 12px 12px",
                        padding: "32px 40px",
                        border: `1px solid ${COLORS.CARD_BORDER}`,
                        borderTop: "none",
                    }}
                >
                    <div
                        style={{
                            fontFamily: FONTS.MONO,
                            fontSize: FONT_SIZES.TERMINAL,
                            color: COLORS.WHITE,
                            padding: "12px 20px",
                            borderRadius: 8,
                            backgroundColor: interpolate(codeHighlight, [0, 1], [0, 0.08]).toString().replace(/^/, "rgba(139,92,246,") + ")",
                            border: `1px solid rgba(139,92,246,${interpolate(codeHighlight, [0, 1], [0, 0.3])})`,
                        }}
                    >
                        <span style={{ color: COLORS.TERMINAL_GREEN }}>export</span>{" "}
                        <span style={{ color: "#F8BD96" }}>OPENAI_BASE_URL</span>
                        <span style={{ color: COLORS.WHITE_DIM }}>=</span>
                        <span style={{ color: "#89B4FA" }}>http://localhost:4000/proxy/openai/v1</span>
                    </div>
                    <div
                        style={{
                            marginTop: 16,
                            fontFamily: FONTS.INTER,
                            fontSize: 16,
                            color: COLORS.WHITE_DIM,
                            opacity: codeHighlight,
                        }}
                    >
                        ↑ That's the entire integration. One line.
                    </div>
                </div>
            </div>

            {/* Flow diagram */}
            {transitionProgress > 0 && (
                <div
                    style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        right: 0,
                        bottom: 0,
                        opacity: transitionProgress,
                    }}
                >
                    {/* Horizontal flow nodes */}
                    {FLOW_NODES.map((node, i) => {
                        const nodeEntrance = spring({
                            frame: Math.max(0, frame - fps * 4 - i * fps * 0.3),
                            fps,
                            config: { damping: 200 },
                            durationInFrames: fps * 0.5,
                        });

                        return (
                            <div
                                key={`h-${i}`}
                                style={{
                                    position: "absolute",
                                    left: node.x - 80,
                                    top: node.y - 30,
                                    opacity: nodeEntrance,
                                    transform: `scale(${interpolate(nodeEntrance, [0, 1], [0.8, 1])})`,
                                    textAlign: "center",
                                    width: 160,
                                }}
                            >
                                <div style={{ fontSize: 40 }}>{node.icon}</div>
                                <div
                                    style={{
                                        fontFamily: FONTS.INTER,
                                        fontSize: 16,
                                        fontWeight: 600,
                                        color: COLORS.WHITE,
                                        marginTop: 8,
                                        whiteSpace: "pre-line",
                                    }}
                                >
                                    {node.label}
                                </div>
                            </div>
                        );
                    })}

                    {/* Horizontal arrows */}
                    {[0, 1].map((i) => {
                        const arrowEntrance = spring({
                            frame: Math.max(0, frame - fps * 4.5 - i * fps * 0.3),
                            fps,
                            config: { damping: 200 },
                            durationInFrames: fps * 0.4,
                        });

                        const startX = FLOW_NODES[i].x + 80;
                        const endX = FLOW_NODES[i + 1].x - 80;

                        return (
                            <div
                                key={`arrow-${i}`}
                                style={{
                                    position: "absolute",
                                    left: startX,
                                    top: FLOW_NODES[i].y + 10,
                                    width: interpolate(arrowEntrance, [0, 1], [0, endX - startX]),
                                    height: 2,
                                    backgroundColor: COLORS.PURPLE,
                                    opacity: arrowEntrance,
                                }}
                            >
                                <div
                                    style={{
                                        position: "absolute",
                                        right: -6,
                                        top: -4,
                                        width: 0,
                                        height: 0,
                                        borderLeft: "10px solid " + COLORS.PURPLE,
                                        borderTop: "5px solid transparent",
                                        borderBottom: "5px solid transparent",
                                    }}
                                />
                            </div>
                        );
                    })}

                    {/* Down flow nodes */}
                    {FLOW_DOWN_NODES.map((node, i) => {
                        const nodeEntrance = spring({
                            frame: Math.max(0, frame - fps * 5.3 - i * fps * 0.3),
                            fps,
                            config: { damping: 200 },
                            durationInFrames: fps * 0.5,
                        });

                        return (
                            <div
                                key={`v-${i}`}
                                style={{
                                    position: "absolute",
                                    left: node.x - 80,
                                    top: node.y - 20,
                                    opacity: nodeEntrance,
                                    transform: `scale(${interpolate(nodeEntrance, [0, 1], [0.8, 1])})`,
                                    textAlign: "center",
                                    width: 160,
                                }}
                            >
                                <div style={{ fontSize: 32 }}>{node.icon}</div>
                                <div style={{ fontFamily: FONTS.INTER, fontSize: 14, fontWeight: 600, color: COLORS.WHITE_DIM, marginTop: 6, whiteSpace: "pre-line" }}>
                                    {node.label}
                                </div>
                            </div>
                        );
                    })}
                </div>
            )}
        </AbsoluteFill>
    );
};
