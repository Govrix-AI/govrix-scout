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

export const TheReveal: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // Phase 1: Shield icon draws (0 - 1.5s)
    const shieldDraw = interpolate(frame, [0, fps * 1.5], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
    });

    // Phase 2: "GOVRIX SCOUT" types in (1.5s - 2.5s)
    const textStart = fps * 1.5;
    const fullTitle = "GOVRIX SCOUT";
    const titleChars = Math.min(
        fullTitle.length,
        Math.floor(Math.max(0, frame - textStart) / 2.5)
    );
    const titleText = fullTitle.slice(0, titleChars);

    // Phase 3: Tagline fades in (3s - 4s)
    const taglineOpacity = interpolate(
        frame,
        [fps * 3, fps * 4],
        [0, 1],
        { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );

    // Phase 4: Everything slides up and shrinks (5s - 6.5s)
    const shrinkProgress = spring({
        frame: Math.max(0, frame - fps * 5),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 1.2,
    });

    const containerScale = interpolate(shrinkProgress, [0, 1], [1, 0.5]);
    const containerY = interpolate(shrinkProgress, [0, 1], [0, -350]);

    // Purple glow
    const glowSize = interpolate(
        frame,
        [fps * 1.2, fps * 2, fps * 4],
        [0, 40, 20],
        { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );

    // Shield SVG path
    const shieldPath = "M12 2 L3 7 L3 13 C3 18 7 22 12 23 C17 22 21 18 21 13 L21 7 Z";
    const checkPath = "M9 12 L11 14 L15 10";
    const totalLength = 80;

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARKER_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Radial gradient background glow */}
            <div
                style={{
                    position: "absolute",
                    width: 800,
                    height: 800,
                    borderRadius: "50%",
                    background: `radial-gradient(circle, ${COLORS.PURPLE_GLOW} 0%, transparent 70%)`,
                    opacity: interpolate(shieldDraw, [0, 0.5, 1], [0, 0.3, 0.5]),
                }}
            />

            <div
                style={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: 24,
                    transform: `translateY(${containerY}px) scale(${containerScale})`,
                }}
            >
                {/* Shield icon */}
                <svg
                    width={120}
                    height={120}
                    viewBox="0 0 24 24"
                    fill="none"
                    style={{ filter: `drop-shadow(0 0 ${glowSize}px ${COLORS.PURPLE})` }}
                >
                    <path
                        d={shieldPath}
                        stroke={COLORS.PURPLE}
                        strokeWidth={1.5}
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeDasharray={totalLength}
                        strokeDashoffset={interpolate(shieldDraw, [0, 0.7], [totalLength, 0], {
                            extrapolateRight: "clamp",
                        })}
                        fill="none"
                    />
                    <path
                        d={checkPath}
                        stroke={COLORS.WHITE}
                        strokeWidth={2}
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeDasharray={20}
                        strokeDashoffset={interpolate(shieldDraw, [0.7, 1], [20, 0], {
                            extrapolateLeft: "clamp",
                            extrapolateRight: "clamp",
                        })}
                        fill="none"
                    />
                </svg>

                {/* Title */}
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.HERO,
                        fontWeight: 800,
                        color: COLORS.WHITE,
                        letterSpacing: 6,
                        textShadow: `0 0 ${glowSize * 0.5}px ${COLORS.PURPLE_GLOW}`,
                    }}
                >
                    {titleText}
                    {titleChars < fullTitle.length && (
                        <span
                            style={{
                                opacity: interpolate(
                                    frame % (fps * 0.5),
                                    [0, fps * 0.25, fps * 0.5],
                                    [1, 0, 1]
                                ),
                                color: COLORS.PURPLE,
                            }}
                        >
                            |
                        </span>
                    )}
                </div>

                {/* Tagline */}
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.BODY,
                        fontWeight: 400,
                        color: COLORS.WHITE_DIM,
                        opacity: taglineOpacity,
                        letterSpacing: 1,
                    }}
                >
                    Open-Source AI Agent Observability, Governance & Compliance
                </div>
            </div>
        </AbsoluteFill>
    );
};
