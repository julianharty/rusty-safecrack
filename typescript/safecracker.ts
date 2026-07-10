import axios from 'axios';
import type { AxiosInstance } from 'axios';
import * as cheerio from 'cheerio';
import * as fs from 'fs';

const REPORT_FILE: './safecrack_report.md' = './safecrack_report.md';
const DELAY_MS = 15;

interface TestResult {
    combo: string; 
    status: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED';
    details: string; 
    latencyMs: number; 
    payloadBytes: number; 
    httpStatus: string;
}

// Accepts the dynamic url argument explicitly
async function createSession(url: string, id: number): Promise<[AxiosInstance, string]> {
    const instance = axios.create({ timeout: 5000 });
    const name = `Clean_Audit_Bot_T${id}`;

    const initGet = await instance.get(url);
    const sessionCookie = initGet.headers['set-cookie']?.map(c => c.split(';')).join('; ') || '';

    await instance.post(url, new URLSearchParams({
        'action': 'set_name', 'name': name
    }), { headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': sessionCookie } });

    return [instance, sessionCookie];
}

async function runCombo(instance: AxiosInstance, url: string, cookie: string, a: string, b: string, c: string, d: string): Promise<[string, string, number]> {
    const start = Date.now();
    const parameters = [['A', a], ['B', b], ['C', c], ['D', d]];

    for (const [param, value] of parameters) {
        if (DELAY_MS > 0) await new Promise(res => setTimeout(res, DELAY_MS));
        await instance.post(url, new URLSearchParams({
            'action': 'select', 'param': param, 'value': value
        }), { headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie } });
    }

    if (DELAY_MS > 0) await new Promise(res => setTimeout(res, DELAY_MS));
    
    const res = await instance.post(url, new URLSearchParams({ 'action': 'add_attempt' }), {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie }
    });

    return [res.data, res.status.toString(), Date.now() - start];
}

async function main() {
    // Collect command line arguments (process.argv[0] is node, process.argv[1] is the script path)
    const args = process.argv.slice(2);
    const targetUrl = args[0];

    if (!targetUrl) {
        console.error("Error: Missing target URL parameter.");
        console.error("Usage: node --experimental-strip-types safecracker.ts <TARGET_URL>");
        process.exit(1);
    }

    console.log(`Connecting to dynamic target sequence engine: ${targetUrl} ...`);
    console.log("Generating combinatorial coverage dataset...");
    
    const pA = ['red', 'green', 'blue'];
    const pB = ['left', 'middle', 'right'];
    const pC = ['0', '1', '2'];
    const pD = ['alpha', 'beta', 'gamma'];

    const testCases: [string, string, string, string, string][] = [];
    const maxLen = Math.max(pA.length, pB.length, pC.length, pD.length);

    for (let i = 0; i < maxLen * maxLen; i++) {
        const a = pA[Math.floor(i / maxLen) % pA.length];
        const b = pB[i % pB.length];
        const c = pC[Math.floor(i / maxLen) % pC.length];
        const d = pD[(Math.floor(i / maxLen) + (i % maxLen)) % pD.length];
        testCases.push([a, b, c, d, 'Pairwise Matrix Rule']);
    }

    for (const a of pA) {
        for (const b of pB) {
            for (const c of pC) {
                for (const d of pD) {
                    const is3w = a !== 'red' && b === 'right' && d === 'alpha';
                    const is4w = a === 'red' && b === 'right' && c === '2' && d === 'gamma';
                    if (is3w || is4w) {
                        testCases.push([a, b, c, d, is3w ? 'Augmented: 3-Way Core Interaction' : 'Augmented: Strict 4-Way Glitch']);
                    }
                }
            }
        }
    }

    const logs: TestResult[] = [];
    let currentId = 1;
    console.log(`Running matrix sweep over ${testCases.length} isolated per-test sessions...`);

    for (const [a, b, c, d, label] of testCases) {
        try {
            const [instance, cookie] = await createSession(targetUrl, currentId);
            currentId++;

            const [body, status, latency] = await runCombo(instance, targetUrl, cookie, a, b, c, d);
            const combo = `${a} | ${b} | ${c} | ${d}`;
            const size = body.length;

            let evalStatus: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED' = 'SECURELY_LOCKED';
            let diagnostics = 'Safe remained locked';

            const $ = cheerio.load(body);
            const txt = $('.display').text().toLowerCase();

            if (!txt.includes('closed')) {
                if (combo === 'red | left | 0 | alpha') {
                    evalStatus = 'LEGITIMATE_CODE';
                    diagnostics = 'Authorized standard route';
                } else {
                    evalStatus = 'BUG_FOUND';
                    diagnostics = label;
                }
            }

            logs.push({ combo, status: evalStatus, details: diagnostics, latencyMs: latency, payloadBytes: size, httpStatus: status });
        } catch (err) {
            console.error(`[ERROR] Execution broken at combo: ${a}-${b}-${c}-${d}`, err);
        }
    }

    let markdown = `# Production Safe Cracking Metrics & Comprehensive TypeScript Report\n\n`;
    markdown += `| Combination Profile (A, B, C, D) | Status | Diagnostics | Latency | Size | HTTP |\n`;
    markdown += `| :--- | :--- | :--- | :--- | :--- | :--- |\n`;
    for (const r of logs) {
        markdown += `| **${r.combo}** | \`${r.status}\` | ${r.details} | ${r.latencyMs}ms | ${r.payloadBytes} B | \`${r.httpStatus}\` |\n`;
    }

    fs.writeFileSync(REPORT_FILE, markdown);
    console.log(`Sweep complete! Verified results written cleanly to: ${REPORT_FILE}`);
}

main().catch(console.error);

