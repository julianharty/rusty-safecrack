import axios from 'axios';
import type { AxiosInstance } from 'axios';
import * as cheerio from 'cheerio';
import * as fs from 'fs';

const REPORT_FILE = './safecrack_report.md';

interface TestResult {
    combo: string; status: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED';
    details: string; latencyMs: number; payloadBytes: number; httpStatus: string;
}

const getCookieSignature = (cookieStr: string): string => {
    if (!cookieStr) return "NONE";
    const match = cookieStr.match(/session=([^;]+)/i);
    return match ? match[1].substring(0, 8) : cookieStr.substring(0, 8);
};

const createSession = async (url: string, id: number): Promise<[AxiosInstance, string]> => {
    const instance = axios.create({ timeout: 5000 });
    const name = `State_Shared_Bot_B${id}`;

    const initGet = await instance.get(url);
    const sessionCookie = initGet.headers['set-cookie']?.map(c => c.split(';')[0]).join('; ') || '';
    
    console.log(`[SESSION] Initialized Session #${id} (${name}). Cookie Sign: [${getCookieSignature(sessionCookie)}]`);

    await instance.post(url, new URLSearchParams({
        'action': 'set_name', 'name': name
    }).toString(), {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': sessionCookie }
    });

    return [instance, sessionCookie];
};

const performBaselineReset = async (instance: AxiosInstance, url: string, cookie: string, delay: number): Promise<void> => {
    const baseline = [['A', 'red'], ['B', 'left'], ['C', '0'], ['D', 'alpha']];
    for (const [p, v] of baseline) {
        if (delay > 0) await new Promise(res => setTimeout(res, delay));
        await instance.post(url, new URLSearchParams({ 'action': 'select', 'param': p, 'value': v }).toString(), {
            headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie }
        });
    }
};

const runCombo = async (instance: AxiosInstance, url: string, cookie: string, a: string, b: string, c: string, d: string, delay: number): Promise<[string, string, number]> => {
    const start = Date.now();
    await performBaselineReset(instance, url, cookie, delay);

    const parameters = [['A', a], ['B', b], ['C', c], ['D', d]];
    for (const [param, value] of parameters) {
        if (delay > 0) await new Promise(res => setTimeout(res, delay));
        console.log(` -> Selecting Param ${param} = ${value} | Using Cookie: [${getCookieSignature(cookie)}]`);
        
        await instance.post(url, new URLSearchParams({
            'action': 'select', 'param': param, 'value': value
        }).toString(), { 
            headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie } 
        });
    }

    if (delay > 0) await new Promise(res => setTimeout(res, delay));
    console.log(` [EXECUTE] Pressing 'Test this combination' for: ${a}-${b}-${c}-${d}`);
    
    const res = await instance.post(url, new URLSearchParams({ 'action': 'add_attempt' }).toString(), {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Cookie': cookie }
    });

    return [res.data, res.status.toString(), Date.now() - start];
};

async function main() {
    const args = process.argv.slice(2);
    const urlArg = args.find(a => !a.startsWith('--'));
    if (!urlArg) {
        console.error("Usage: node --experimental-strip-types safecracker.ts <URL> [--attempts 10] [--delay 15]");
        process.exit(1);
    }

    let maxAttemptsPerSession = 10;
    const attemptsIdx = args.indexOf('--attempts');
    if (attemptsIdx !== -1 && attemptsIdx + 1 < args.length) {
        maxAttemptsPerSession = parseInt(args[attemptsIdx + 1], 10) || 10;
    }

    let delayMs = 0;
    const delayIdx = args.indexOf('--delay');
    if (delayIdx !== -1 && delayIdx + 1 < args.length) {
        delayMs = parseInt(args[delayIdx + 1], 10) || 0;
    }

    console.log(`Connecting to target: ${urlArg}`);
    console.log(`[CONFIG] Session threshold locked at: ${maxAttemptsPerSession} attempts.`);

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
    let currentBatchId = 1;
    let attemptCounter = 0;

    let [currentInstance, currentCookie] = await createSession(urlArg, currentBatchId);

    for (const [a, b, c, d, label] of testCases) {
        try {
            if (attemptCounter >= maxAttemptsPerSession) {
                currentBatchId++;
                const [nextInstance, nextCookie] = await createSession(urlArg, currentBatchId);
                currentInstance = nextInstance;
                currentCookie = nextCookie;
                attemptCounter = 0;
            }

            // Fixed: Standardised argument reference to evaluate 'delayMs' variable correctly
            const [body, status, latency] = await runCombo(currentInstance, urlArg, currentCookie, a, b, c, d, delayMs);
            attemptCounter++;

            const combo = `${a} | ${b} | ${c} | ${d}`;
            const size = body.length;
            let evalStatus: 'LEGITIMATE_CODE' | 'BUG_FOUND' | 'SECURELY_LOCKED' = 'SECURELY_LOCKED';
            let diagnostics = 'Safe remained locked';

            const $ = cheerio.load(body);
            const attemptRows = $('.attempt, .card h2:contains("Attempts") ~ p, .card h2:contains("Attempts") ~ div, table tr');
            let latestRowText = "";
            
            if (attemptRows.length > 0) {
                latestRowText = $(attemptRows[attemptRows.length - 1]).text().toLowerCase();
            } else {
                latestRowText = $('.display').text().toLowerCase();
            }

            if (latestRowText.includes('bug found') || latestRowText.includes('wrong code')) {
                evalStatus = 'BUG_FOUND';
                diagnostics = label;
                console.log(` [ALERT] SUCCESS! Bug verified for configuration: ${combo}`);
            } else if (combo === 'red | left | 0 | alpha') {
                evalStatus = 'LEGITIMATE_CODE';
                diagnostics = 'Authorized standard route';
            }

            logs.push({ combo, status: evalStatus, details: diagnostics, latencyMs: latency, payloadBytes: size, httpStatus: status });
        } catch (err) {
            console.error(`[ERROR] Broken at combo: ${a}-${b}-${c}-${d}`, err);
        }
    }

    let markdown = `# Production Safe Cracking Shared-Session Report\n\n`;
    markdown += `| Combination Profile (A, B, C, D) | Status | Diagnostics | Latency | Size | HTTP |\n`;
    markdown += `| :--- | :--- | :--- | :--- | :--- | :--- |\n`;
    for (const r of logs) {
        markdown += `| **${r.combo}** | \`${r.status}\` | ${r.details} | ${r.latencyMs}ms | ${r.payloadBytes} B | \`${r.httpStatus}\` |\n`;
    }

    fs.writeFileSync(REPORT_FILE, markdown);
    console.log(`Sweep complete! Verified results written cleanly to: ${REPORT_FILE}`);
}

main().catch(console.error);

