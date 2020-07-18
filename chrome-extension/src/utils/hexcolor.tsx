export default class HexColor {
    public r: number;
    public g: number;
    public b: number;

    constructor(css: string) {
        let parts = css
            .toLowerCase()
            .match(/#(?<r>[a-z0-9]{2})(?<g>[a-z0-9]{2})(?<b>[a-z0-9]{2})/)
            ?.groups;

        let rHex = parts?.r;
        let gHex = parts?.g;
        let bHex = parts?.b;

        if (rHex === undefined || gHex === undefined || bHex === undefined) {
            throw new Error('Failed to parse hex color.');
        }

        let rDec = parseInt(rHex, 16);
        let gDec = parseInt(gHex, 16);
        let bDec = parseInt(bHex, 16);

        if (Number.isNaN(rDec) || Number.isNaN(gDec) || Number.isNaN(bDec)) {
            throw new Error('Failed to parse hex color.');
        }

        this.r = rDec;
        this.g = gDec;
        this.b = bDec;
    }

    get css(): string {
        return '#' + [this.r, this.g, this.b]
            .map(n => n.toString(16).padStart(2))
            .join('');
    }
}