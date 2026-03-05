import React from "react";
import { AbsoluteFill, Sequence, useVideoConfig } from "remotion";
import { TransitionSeries, linearTiming } from "@remotion/transitions";
import { fade } from "@remotion/transitions/fade";

import { ColdOpen } from "./scenes/ColdOpen";
import { FiveQuestions } from "./scenes/FiveQuestions";
import { TheWeight } from "./scenes/TheWeight";
import { TheReveal } from "./scenes/TheReveal";
import { DashboardAgents } from "./scenes/DashboardAgents";
import { DashboardEvents } from "./scenes/DashboardEvents";
import { DashboardCosts } from "./scenes/DashboardCosts";
import { TheHow } from "./scenes/TheHow";
import { ComplianceShield } from "./scenes/ComplianceShield";
import { TheScale } from "./scenes/TheScale";
import { TheClose } from "./scenes/TheClose";

/*
 * GOVRIX SCOUT — 2-Minute Launch Video
 * 30fps × 120s = 3600 frames
 *
 * ACT 1 — "THE STORY" (0:00 – 0:35)
 *   Scene 1: ColdOpen        0:00 – 0:08  (240 frames)
 *   Scene 2: FiveQuestions    0:08 – 0:22  (420 frames)
 *   Scene 3: TheWeight        0:22 – 0:35  (390 frames)
 *
 * ACT 2 — "THE PRODUCT" (0:35 – 1:30)
 *   Scene 4: TheReveal        0:35 – 0:45  (300 frames)
 *   Scene 5: DashboardAgents  0:45 – 0:52  (210 frames)
 *   Scene 6: DashboardEvents  0:52 – 1:00  (240 frames)
 *   Scene 7: DashboardCosts   1:00 – 1:05  (150 frames)
 *   Scene 8: TheHow           1:05 – 1:20  (450 frames)
 *   Scene 9: ComplianceShield 1:20 – 1:30  (300 frames)
 *
 * ACT 3 — "THE FUTURE" (1:30 – 2:00)
 *   Scene 10: TheScale        1:30 – 1:45  (450 frames)
 *   Scene 11: TheClose        1:45 – 2:00  (450 frames)
 *
 * Transition overlap: 15 frames fade between each scene
 * Total scenes: 10 transitions × 15 frames = 150 frames subtracted
 * Raw sum: 3600 + 150 = 3750 frames needed
 */

const FADE_DURATION = 15; // frames

export const GovrixLaunchVideo: React.FC = () => {
    const { fps } = useVideoConfig();

    return (
        <AbsoluteFill style={{ backgroundColor: "#050508" }}>
            <TransitionSeries>
                {/* === ACT 1 — "THE STORY" === */}

                {/* Scene 1: Cold Open (0:00 – 0:08) */}
                <TransitionSeries.Sequence durationInFrames={255}>
                    <ColdOpen />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 2: Five Questions (0:08 – 0:22) */}
                <TransitionSeries.Sequence durationInFrames={435}>
                    <FiveQuestions />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 3: The Weight (0:22 – 0:35) */}
                <TransitionSeries.Sequence durationInFrames={405}>
                    <TheWeight />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* === ACT 2 — "THE PRODUCT" === */}

                {/* Scene 4: The Reveal (0:35 – 0:45) */}
                <TransitionSeries.Sequence durationInFrames={315}>
                    <TheReveal />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 5: Dashboard Agents (0:45 – 0:52) */}
                <TransitionSeries.Sequence durationInFrames={225}>
                    <DashboardAgents />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 6: Dashboard Events (0:52 – 1:00) */}
                <TransitionSeries.Sequence durationInFrames={255}>
                    <DashboardEvents />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 7: Dashboard Costs (1:00 – 1:05) */}
                <TransitionSeries.Sequence durationInFrames={165}>
                    <DashboardCosts />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 8: The How (1:05 – 1:20) */}
                <TransitionSeries.Sequence durationInFrames={465}>
                    <TheHow />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 9: Compliance Shield (1:20 – 1:30) */}
                <TransitionSeries.Sequence durationInFrames={315}>
                    <ComplianceShield />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* === ACT 3 — "THE FUTURE" === */}

                {/* Scene 10: The Scale (1:30 – 1:45) */}
                <TransitionSeries.Sequence durationInFrames={465}>
                    <TheScale />
                </TransitionSeries.Sequence>

                <TransitionSeries.Transition
                    presentation={fade()}
                    timing={linearTiming({ durationInFrames: FADE_DURATION })}
                />

                {/* Scene 11: The Close (1:45 – 2:00) */}
                <TransitionSeries.Sequence durationInFrames={465}>
                    <TheClose />
                </TransitionSeries.Sequence>
            </TransitionSeries>
        </AbsoluteFill>
    );
};
