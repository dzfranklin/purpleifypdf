export enum Quality {
    Extreme = "Extreme",
    High = "High",
    Normal = "Normal",
    Low = "Low",
    ExtraLow = "ExtraLow",
}

export function qualityFromString(s: string): Quality {
    switch (s.toLowerCase()) {
        case 'extreme': return Quality.Extreme;
        case 'high': return Quality.High;
        case 'normal': return Quality.Normal;
        case 'low': return Quality.Low;
        case 'extralow': return Quality.ExtraLow;
        default: throw new Error(`Failed to parse invalid quality: '${s}'`);
    }
}