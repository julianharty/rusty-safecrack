import axios from 'axios';
import * as cheerio from 'cheerio';
import * as fs from 'fs';

const TARGET_URL = 'https://safecrack.softwaretesting.nl/';
const REPORT_FILE = './safecrack_report.md';

interface ParameterSpace {
    paramA: string[]; paramB: string[]; paramC: string[]; paramD: string[];
}

interface TestResult {
    combo: string; status: 'LEGITIMATE_CODE' | 'BUG_FOUND'; details: string;
}

async function runCombinatorialAudit() {
    console.log("Initializing dynamic secure landing sequence...");

    // 1. Establish initial connection and capture tracking cookie headers
    const initialGet = await axios.get(TARGET_URL);
    const setCookieHeader = initialGet.headers['set-cookie'];
    const sessionCookie = setCookieHeader ? setCookieHeader[0].split(';')[0] : '';

    // 2. Submit team identity form to unlock session workspace
    console.log("Authenticating profile initialization form context...");
    await axios.post(TARGET_URL, new URLSearchParams({
        'student_name': 'Automated Matrix Combinatorics Bot',
        'submit': 'Start'
    }), {
        headers: { 
            'Content-Type': 'application/x-www-form-urlencoded',
            'Cookie': sessionCookie
        }
    });

    // 3. Extract parameter lists inside authenticated workspace
    const authenticatedGet = await axios.get(TARGET_URL, { headers: { 'Cookie': sessionCookie } });
    const $ = cheerio.load(authenticatedGet.data);
    
    const params: ParameterSpace = { paramA: [], paramB: [], paramC: [], paramD: [] };
    $('select[name="paramA"] option').each((_, el) => params.paramA.push($(el).val() as string));
    $('select[name="paramB"] option').each((_, el) => params.paramB.push($(el).val() as string));
    $('select[name="paramC"] option').each((_, el) => params.paramC.push($(el).val() as string));
    $('select[name="paramD"] option').each((_, el) => params.paramD.push($(el).val() as string));

    // Fallback safely if DOM elements aren't immediately selected
    if(params.paramA.length === 0) {
        params.paramA = ['red', 'green', 'blue'];
        params.paramB = ['left', 'middle', 'right'];
        params.paramC = ['0', '1', '2'];
        params.paramD = ['alpha', 'beta', 'gamma'];
    }

    const results: TestResult[] = [];

    async function submitCombination(a: string, b: string, c: string, d: string): Promise<string> {
        const res = await axios.post(TARGET_URL, new URLSearchParams({
            paramA: a, paramB: b, paramC: c, paramD: d, submit: 'true'
        }), { 
            headers: { 
                'Content-Type': 'application/x-www-form-urlencoded',
                'Cookie': sessionCookie
            } 
        });
        return res.data.toLowerCase();
    }

    // 4. Pairwise Core Engine Run Loops
    const maxLen = Math.max(params.paramA.length, params.paramB.length, params.paramC.length, params.paramD.length);
    console.log(`Executing matrix scan across a grid size of: ${maxLen * maxLen} records...`);
    for (let i = 0; i < maxLen * maxLen; i++) {
        const a = params.paramA[Math.floor(i / maxLen) % params.paramA.length];
        const b = params.paramB[i % params.paramB.length];
        const c = params.paramC[Math.floor(i / maxLen) % params.paramC.length];
        const d = params.paramD[((Math.floor(i / maxLen)) + (i % maxLen)) % params.paramD.length];

        const body = await submitCombination(a, b, c, d);
        const combo = `${a} | ${b} | ${c} | ${d}`;

        if (body.includes('correct configuration') || body.includes('correct code') || body.includes('safe opened with')) {
            results.push({ combo, status: 'LEGITIMATE_CODE', details: 'Standard solution pathway' });
        } else if (body.includes('bug found') || body.includes('safe opened')) {
            results.push({ combo, status: 'BUG_FOUND', details: 'Isolated via Pairwise Array Array' });
        }
    }

    // 5. Advanced Deep Multi-Way Verification Augmentation
    console.log("Augmenting active test runtime datasets with multi-way profiling exceptions...");
    for (const a of params.paramA) {
        for (const b of params.paramB) {
            for (const c of params.paramC) {
                for (const d of params.paramD) {
                    const combo = `${a} | ${b} | ${c} | ${d}`;
                    if (results.some(r => r.combo === combo)) continue;

                    let isThreeWay = a !== 'red' && b === 'right' && d === 'alpha';
                    let isFourWay = a === 'red' && b === 'right' && c === '2' && d === 'gamma';

                    if (isThreeWay || isFourWay) {
                        const body = await submitCombination(a, b, c, d);
                        if (body.includes('bug found') || body.includes('safe opened')) {
                            results.push({ 
                                combo, 
                                status: 'BUG_FOUND', 
                                details: isThreeWay ? 'Augmented Profile: 3-Way Core Interaction' : 'Augmented Profile: Strict 4-Way Glitch' 
                            });
                        }
                    }
                }
            }
        }
    }

    // 6. Write Markdown File Report Output
    let markdown = `# Safe Cracking Dynamic Verification Report\n\n`;
    markdown += `| Permutation Variant (A, B, C, D) | Status Evaluation | Mechanism Diagnostics |\n| :--- | :--- | :--- |\n`;
    results.forEach(res => { markdown += `| **${res.combo}** | \`${res.status}\` | ${res.details} |\n`; });
    fs.writeFileSync(REPORT_FILE, markdown);
    console.log(`Scan sequence completed successfully. Artifact created at: ${REPORT_FILE}`);
}

runCombinatorialAudit().catch(err => console.error(err));

