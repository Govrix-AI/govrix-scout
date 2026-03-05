import React from "react";
import {
    AbsoluteFill,
    interpolate,
    useCurrentFrame,
    useVideoConfig,
} from "remotion";
import { COLORS, FONT_SIZES } from "../utils/colors";
import { FONTS } from "../utils/fonts";

const COMMAND = "> audit_report --quarter Q1 --department engineering";
const ERROR_LINES = [
    "ERROR: No AI agent activity logs found.",
    "       0 agents tracked. 0 events recorded.",
    "       Audit status: FAILED",
];

const CHAR_SPEED = 1.5; // frames per character

export const ColdOpen: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // Cursor blink
    const cursorOpacity = interpolate(
        frame % (fps * 0.6),
        [0, fps * 0.3, fps * 0.6],
        [1, 0, 1],
        { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );

    // Typing the command
    const commandChars = Math.min(
        COMMAND.length,
        Math.floor(frame / CHAR_SPEED)
    );
    const commandText = COMMAND.slice(0, commandChars);
    const commandDone = commandChars >= COMMAND.length;

    // Wait 0.8s after command, then reveal error
    const errorStartFrame = COMMAND.length * CHAR_SPEED + fps * 0.8;
    const errorProgress = Math.max(0, frame - errorStartFrame);

    // Error lines appear one at a time
    const errorLine0Chars = Math.min(
        ERROR_LINES[0].length,
        Math.floor(errorProgress / CHAR_SPEED)
    );
    const errorLine1Start =
        ERROR_LINES[0].length * CHAR_SPEED + fps * 0.15;
    const errorLine1Chars = Math.min(
        ERROR_LINES[1].length,
        Math.floor(Math.max(0, errorProgress - errorLine1Start) / CHAR_SPEED)
    );
    const errorLine2Start =
        errorLine1Start + ERROR_LINES[1].length * CHAR_SPEED + fps * 0.15;
    const errorLine2Chars = Math.min(
        ERROR_LINES[2].length,
        Math.floor(Math.max(0, errorProgress - errorLine2Start) / CHAR_SPEED)
    );

    // Scanline effect
    const scanlineY = interpolate(frame, [0, fps * 4], [0, 100], {
        extrapolateRight: "extend",
    });

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.TERMINAL_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Scanline overlay */}
            <div
                style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: `repeating-linear-gradient(
            0deg,
            transparent,
            transparent 2px,
            rgba(0,0,0,0.15) 2px,
            rgba(0,0,0,0.15) 4px
          )`,
                    pointerEvents: "none",
                    zIndex: 10,
                }}
            />

            {/* Terminal window */}
            <div
                style={{
                    width: 1200,
                    padding: 60,
                    fontFamily: FONTS.MONO,
                    fontSize: FONT_SIZES.TERMINAL,
                    lineHeight: 1.8,
                }}
            >
                {/* Command line */}
                <div style={{ color: COLORS.TERMINAL_GREEN }}>
                    <span>{commandText}</span>
                    {!commandDone && (
                        <span style={{ opacity: cursorOpacity, color: COLORS.WHITE }}>
                            █
                        </span>
                    )}
                </div>

                {/* Error output */}
                {commandDone && errorProgress > 0 && (
                    <div style={{ marginTop: 24 }}>
                        <div style={{ color: COLORS.RED }}>
                            {ERROR_LINES[0].slice(0, errorLine0Chars)}
                        </div>
                        {errorLine1Chars > 0 && (
                            <div style={{ color: COLORS.WHITE_DIM }}>
                                {ERROR_LINES[1].slice(0, errorLine1Chars)}
                            </div>
                        )}
                        {errorLine2Chars > 0 && (
                            <div
                                style={{
                                    color: COLORS.RED,
                                    fontWeight: 700,
                                }}
                            >
                                {ERROR_LINES[2].slice(0, errorLine2Chars)}
                            </div>
                        )}
                    </div>
                )}

                {/* Cursor after error */}
                {commandDone && errorLine2Chars >= ERROR_LINES[2].length && (
                    <div style={{ marginTop: 24 }}>
                        <span
                            style={{
                                color: COLORS.TERMINAL_GREEN,
                                opacity: cursorOpacity,
                            }}
                        >
                            {">"} █
                        </span>
                    </div>
                )}
            </div>
        </AbsoluteFill>
    );
};
