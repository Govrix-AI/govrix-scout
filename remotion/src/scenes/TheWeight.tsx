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

export const TheWeight: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // Phase 1: Cards dissolve (0 - 1s)
    const dissolveProgress = interpolate(frame, [0, fps * 0.8], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
    });

    // Phase 2: Purple text appears (1.5s - 3s)
    const purpleTextEntrance = spring({
        frame: Math.max(0, frame - fps * 1.5),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.8,
    });

    // Phase 3: "Until now" appears (5s - 7s)
    const untilNowEntrance = spring({
        frame: Math.max(0, frame - fps * 5),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.8,
    });

    // Glow pulse on "Until now"
    const glowPulse = interpolate(
        Math.max(0, frame - fps * 5.5),
        [0, fps * 0.5, fps * 1, fps * 1.5],
        [0, 1, 0.3, 0.6],
        { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );

    // Dissolving card fragments
    const fragments = Array.from({ length: 12 }, (_, i) => {
        const angle = (i / 12) * Math.PI * 2;
        const distance = interpolate(dissolveProgress, [0, 1], [0, 600]);
        const rotation = interpolate(dissolveProgress, [0, 1], [0, (i % 2 === 0 ? 1 : -1) * 180]);
        const fragOpacity = interpolate(dissolveProgress, [0, 0.6, 1], [0.8, 0.3, 0], {
            extrapolateRight: "clamp",
        });
        return {
            x: Math.cos(angle) * distance,
            y: Math.sin(angle) * distance,
            rotation,
            opacity: fragOpacity,
            width: 60 + (i % 3) * 40,
            height: 20 + (i % 2) * 15,
        };
    });

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARKER_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Dissolving fragments */}
            {dissolveProgress < 1 &&
                fragments.map((frag, i) => (
                    <div
                        key={i}
                        style={{
                            position: "absolute",
                            width: frag.width,
                            height: frag.height,
                            backgroundColor: COLORS.CARD_BG,
                            border: `1px solid ${COLORS.CARD_BORDER}`,
                            borderRadius: 4,
                            opacity: frag.opacity,
                            transform: `translate(${frag.x}px, ${frag.y}px) rotate(${frag.rotation}deg)`,
                        }}
                    />
                ))}

            {/* "They couldn't answer a single one." */}
            <div
                style={{
                    position: "absolute",
                    opacity: purpleTextEntrance,
                    transform: `scale(${interpolate(purpleTextEntrance, [0, 1], [0.8, 1])})`,
                }}
            >
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.TITLE,
                        fontWeight: 700,
                        color: COLORS.PURPLE,
                        textAlign: "center",
                        letterSpacing: -1,
                    }}
                >
                    They couldn't answer a single one.
                </div>
            </div>

            {/* "Until now." */}
            <div
                style={{
                    position: "absolute",
                    top: "58%",
                    opacity: untilNowEntrance,
                    transform: `scale(${interpolate(untilNowEntrance, [0, 1], [0.85, 1])})`,
                }}
            >
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.HERO,
                        fontWeight: 800,
                        color: COLORS.WHITE,
                        textAlign: "center",
                        textShadow: `0 0 ${interpolate(glowPulse, [0, 1], [0, 60])}px rgba(255,255,255,${glowPulse * 0.5})`,
                    }}
                >
                    Until now.
                </div>
            </div>
        </AbsoluteFill>
    );
};
