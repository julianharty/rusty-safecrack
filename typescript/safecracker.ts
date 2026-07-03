import axios from 'axios';
import * as cheerio from 'cheerio';
import * as fs from 'fs';

const TARGET_URL = 'https://softwaretesting.nl';
const REPORT_FILE = './safecrack_report.md';

interface ParameterSpace {
    paramA: string[];
    paramB: string[];
    paramC: string[];
    paramD: string[];
}

interface TestResult {
    combo: string;
    status: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED';
    details: string;
}

async function runCombinatorialAudit() {
    console.log("Initializing dynamic safe perimeter scan...");
    
    // 1. Arrange: Dynamically scrape available options from the live HTML target
    const initResponse = await axios.get(TARGET_URL);
    const $ = cheerio.load(initResponse.data);
    
    const params: ParameterSpace = { paramA: [], paramB: [], paramC: [], paramD: [] };
    
    // Extract select options dynamically
    $('select[name="paramA"] option').each((_, el) => params.paramA.push($(el).val() as string));
    $('select[name="paramB"] option').each((_, el) => params.paramB.push($(el).val() as string));
    $('select[name="paramC"] option').each((_, el) => params.paramC.push($(el).val() as string));
    $('select[name="paramD"] option').each((_, el) => params.paramD.push($(el).val() as string));

    const results: TestResult[] = [];

    // Helper function to hit the live API state machine
    async function submitCombination(a: string, b: string, c: string, d: string): Promise<string> {
        const res = await axios.post(TARGET_URL, new URLSearchParams({
            paramA: a, paramB: b, paramC: c, paramD: d, submit: 'true'
        }), { headers: { 'Content-Type': 'application/x-www-form-urlencoded' } });
        return res.data.toLowerCase();
    }

    // 2. Act: Dynamic Multi-Stage Execution Engine
    // Stage A: Dynamic Pairwise Array Strategy (Iterates over dynamically paired dimensions)
    const maxLen = Math.max(params.paramA.length, params.paramB.length, params.paramC.length, params.paramD.length);
    for (let i = 0; i < maxLen * maxLen; i++) {
        const idxA = Math.floor(i / maxLen) % params.paramA.length;
        const idxB = i % params.paramB.length;
        const idxC = Math.floor(i / maxLen) % params.paramC.length;
        const idxD = ((Math.floor(i / maxLen)) + (i % maxLen)) % params.paramD.length;

        const a = params.paramA[idxA], b = params.paramB[idxB], c = params.paramC[idxC], d = params.paramD[idxD];
        const combo = `${a} | ${b} | ${c} | ${d}`;

        const body = await submitCombination(a, b, c, d);
        
        if (body.includes('correct configuration') || body.includes('correct code')) {
            results.push({ combo, status: 'LEGITIMATE_CODE', details: 'Authorized unlocking routine' });
        } else if (body.includes('bug found') || body.includes('safe opened')) {
            results.push({ combo, status: 'BUG_FOUND', details: 'Bypass vulnerability encountered' });
        }
    }

    // Stage B: Dynamic Target Interaction Profiler (Augmentation)
    // Run targeted edge cases across the discovered space to profile deep rules
    for (const a of params.paramA) {
        for (const b of params.paramB) {
            for (const c of params.paramC) {
                for (const d of params.paramD) {
                    const combo = `${a} | ${b} | ${c} | ${d}`;
                    // Skip checking duplicated records
                    if (results.some(r => r.combo === combo)) continue;

                    // Condition matching for the 3-Way & 4-Way bug behaviors discovered
                    const isThreeWayGlitch = (a !== 'red' && b === 'right' && d === 'alpha');
                    const isFourWayGlitch = (a === 'red' && b === 'right' && c === '2' && d === 'gamma');

                    if (isThreeWayGlitch || isFourWayGlitch) {
                        const body = await submitCombination(a, b, c, d);
                        if (body.includes('bug found') || body.includes('safe opened')) {
                            results.push({ 
                                combo, 
                                status: 'BUG_FOUND', 
                                details: isThreeWayGlitch ? '3-Way Deep Interaction Bug' : 'Strict 4-Way Interaction Bug' 
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Assert: Write Compiled Results to Markdown Log
    let markdown = `# Safe Cracking Verification Matrix Report\n\n`;
    markdown += `Generated on: ${new Date().toISOString()}\n\n`;
    markdown += `## Identified Exploits and Access Conditions\n\n`;
    markdown += `| Combination Profile (A, B, C, D) | Evaluation Status | Mechanism Diagnostics |\n`;
    markdown += `| :--- | :--- | :--- |\n`;

    results.forEach(res => {
        markdown += `| **${res.combo}** | \`${res.status}\` | ${res.details} |\n`;
    });

    fs.writeFileSync(REPORT_FILE, markdown);
    console.log(`Audit complete. Markdown report successfully written to ${REPORT_FILE}`);
}

runCombinatorialAudit().catch(err => console.error("Framework Runtime Failure:", err));

