import { Composition } from "remotion";
import { GovrixLaunchVideo } from "./GovrixLaunchVideo";

// Total duration calculation:
// 11 scenes with 10 fade transitions × 15 frames each
// Sum of all scene durations: 255+435+405+315+225+255+165+465+315+465+465 = 3765
// Minus transitions: 10 × 15 = 150
// Total: 3765 - 150 = 3615 frames ≈ 120.5s at 30fps
// Close enough to 2:00 — we target 3600 frames for exactly 120s

export const RemotionRoot: React.FC = () => {
    return (
        <Composition
            id="GovrixLaunchVideo"
            component={GovrixLaunchVideo}
            durationInFrames={3615}
            fps={30}
            width={1920}
            height={1080}
        />
    );
};
