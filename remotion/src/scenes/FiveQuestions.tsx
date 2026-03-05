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

const QUESTIONS = [
    { text: '"How many AI agents do you operate?"', from: "left" },
    { text: '"What data did they access?"', from: "right" },
    { text: '"Can you reconstruct what happened on March 3rd?"', from: "bottom" },
    { text: '"Was any customer PII exposed?"', from: "center" },
    { text: '"Where is your audit trail?"', from: "center", isLast: true },
] as const;

export const FiveQuestions: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    const staggerDelay = fps * 0.7; // 0.7s between each card

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARK_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Subtle grid background */}
            <div
                style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    backgroundImage: `
            linear-gradient(rgba(255,255,255,0.02) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255,255,255,0.02) 1px, transparent 1px)
          `,
                    backgroundSize: "60px 60px",
                }}
            />

            <div
                style={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: 20,
                    width: 1200,
                }}
            >
                {QUESTIONS.map((q, i) => {
                    const delay = i * staggerDelay;
                    const localFrame = Math.max(0, frame - delay);

                    const entrance = spring({
                        frame: localFrame,
                        fps,
                        config: { damping: 200 },
                        durationInFrames: fps * 0.6,
                    });

                    // Direction-based offset
                    let translateX = 0;
                    let translateY = 0;
                    if (q.from === "left") translateX = interpolate(entrance, [0, 1], [-400, 0]);
                    if (q.from === "right") translateX = interpolate(entrance, [0, 1], [400, 0]);
                    if (q.from === "bottom") translateY = interpolate(entrance, [0, 1], [200, 0]);

                    const opacity = interpolate(entrance, [0, 1], [0, 1]);
                    const scale = interpolate(entrance, [0, 1], [0.9, 1]);

                    const isLast = q.isLast;
                    const glowOpacity = isLast
                        ? interpolate(
                            frame % (fps * 1.5),
                            [0, fps * 0.75, fps * 1.5],
                            [0.3, 0.7, 0.3],
                            { extrapolateRight: "clamp" }
                        )
                        : 0;

                    return (
                        <div
                            key={i}
                            style={{
                                opacity,
                                transform: `translate(${translateX}px, ${translateY}px) scale(${scale})`,
                                background: COLORS.CARD_BG,
                                border: `1px solid ${isLast ? COLORS.RED : COLORS.CARD_BORDER}`,
                                borderRadius: 16,
                                padding: "24px 48px",
                                width: "100%",
                                boxShadow: isLast
                                    ? `0 0 30px ${interpolate(glowOpacity, [0, 1], [0, 1])}px ${COLORS.RED_GLOW}`
                                    : "none",
                            }}
                        >
                            <span
                                style={{
                                    color: COLORS.WHITE_DIM,
                                    fontFamily: FONTS.INTER,
                                    fontSize: 14,
                                    textTransform: "uppercase",
                                    letterSpacing: 2,
                                }}
                            >
                                Question {i + 1}
                            </span>
                            <div
                                style={{
                                    color: COLORS.WHITE,
                                    fontFamily: FONTS.INTER,
                                    fontSize: FONT_SIZES.BODY,
                                    fontWeight: 600,
                                    marginTop: 8,
                                }}
                            >
                                {q.text}
                            </div>
                        </div>
                    );
                })}
            </div>
        </AbsoluteFill>
    );
};
