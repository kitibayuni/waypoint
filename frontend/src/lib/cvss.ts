// CVSS v3.1 Base Score, per the official spec's Roundup algorithm.
// https://www.first.org/cvss/v3.1/specification-document

export interface CvssMetrics {
	AV: 'N' | 'A' | 'L' | 'P';
	AC: 'L' | 'H';
	PR: 'N' | 'L' | 'H';
	UI: 'N' | 'R';
	S: 'U' | 'C';
	C: 'N' | 'L' | 'H';
	I: 'N' | 'L' | 'H';
	A: 'N' | 'L' | 'H';
}

const AV_VALUES: Record<CvssMetrics['AV'], number> = { N: 0.85, A: 0.62, L: 0.55, P: 0.2 };
const AC_VALUES: Record<CvssMetrics['AC'], number> = { L: 0.77, H: 0.44 };
const UI_VALUES: Record<CvssMetrics['UI'], number> = { N: 0.85, R: 0.62 };
const CIA_VALUES: Record<CvssMetrics['C'], number> = { H: 0.56, L: 0.22, N: 0 };

function prValue(pr: CvssMetrics['PR'], scope: CvssMetrics['S']): number {
	if (pr === 'N') return 0.85;
	if (pr === 'L') return scope === 'C' ? 0.68 : 0.62;
	return scope === 'C' ? 0.5 : 0.27;
}

function roundUp(value: number): number {
	const intInput = Math.round(value * 100000);
	if (intInput % 10000 === 0) {
		return intInput / 100000;
	}
	return (Math.floor(intInput / 10000) + 1) / 10;
}

export interface CvssResult {
	score: number;
	vector: string;
	severity: 'none' | 'low' | 'medium' | 'high' | 'critical';
}

export function severityForScore(score: number): CvssResult['severity'] {
	if (score === 0) return 'none';
	if (score < 4) return 'low';
	if (score < 7) return 'medium';
	if (score < 9) return 'high';
	return 'critical';
}

export function calculateCvss(m: CvssMetrics): CvssResult {
	const iss = 1 - (1 - CIA_VALUES[m.C]) * (1 - CIA_VALUES[m.I]) * (1 - CIA_VALUES[m.A]);
	const impact = m.S === 'U' ? 6.42 * iss : 7.52 * (iss - 0.029) - 3.25 * Math.pow(iss - 0.02, 15);

	const exploitability = 8.22 * AV_VALUES[m.AV] * AC_VALUES[m.AC] * prValue(m.PR, m.S) * UI_VALUES[m.UI];

	let score: number;
	if (impact <= 0) {
		score = 0;
	} else if (m.S === 'U') {
		score = roundUp(Math.min(impact + exploitability, 10));
	} else {
		score = roundUp(Math.min(1.08 * (impact + exploitability), 10));
	}

	const vector = `CVSS:3.1/AV:${m.AV}/AC:${m.AC}/PR:${m.PR}/UI:${m.UI}/S:${m.S}/C:${m.C}/I:${m.I}/A:${m.A}`;

	return { score, vector, severity: severityForScore(score) };
}
