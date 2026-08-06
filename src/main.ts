// 前端進入點。
//
// 這裡只負責畫面與事件；所有二進位處理都在 Rust（src-tauri/src/patch/）。
// 這個分工是刻意的，與 RICH2_EDITOR 相反——patcher 沒有瀏覽器版的需求，
// 把引擎放 Rust 就不必開放檔案系統權限給前端（見 DEVELOPER.md §9.2）。

import './style.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

/** 與 Rust 端 `LogPayload` 對應。 */
interface LogEvent {
    level: string;
    message: string;
    /** 有值代表這是一個步驟的開始，要推進進度條 */
    step: number | null;
    total: number;
}

const el = <T extends HTMLElement>(id: string): T => {
    const node = document.getElementById(id);
    if (!node) throw new Error(`找不到元素 #${id}`);
    return node as T;
};

const dirInput = el<HTMLInputElement>('dir');
const browseBtn = el<HTMLButtonElement>('browse');
const startBtn = el<HTMLButtonElement>('start');
const bar = el<HTMLDivElement>('bar');
const logBox = el<HTMLElement>('log');

let running = false;

/** 日誌等級對應的顏色。等級字串與 Python 版共用同一套。 */
const LEVEL_STYLE: Record<string, string> = {
    INFO: 'text-on-surface-variant',
    WARN: 'text-amber-300/80',
    ERROR: 'text-error',
    FATAL: 'text-error font-bold',
    SUCCESS: 'text-tertiary',
    DONE: 'text-tertiary font-bold',
};

function clearLog(): void {
    logBox.replaceChildren();
}

function appendLog(level: string, message: string): void {
    const line = document.createElement('p');
    line.className = `flex gap-2 ${LEVEL_STYLE[level] ?? 'text-on-surface-variant'}`;

    // 等級欄固定寬度，訊息才會對齊；長路徑換行時也會掛在訊息欄下方而不是頂到最左邊
    const tag = document.createElement('span');
    tag.className = 'w-14 shrink-0 text-on-surface-variant/40';
    tag.textContent = level.padEnd(7, ' ');

    const body = document.createElement('span');
    body.className = 'flex-1 min-w-0 break-all';
    body.textContent = message;

    line.append(tag, body);
    logBox.append(line);
    scrollToBottom();
}

/** 執行摘要不需要等級欄，整行留給文字。 */
function appendSummary(message: string): void {
    const line = document.createElement('p');
    line.className = 'break-all text-on-surface';
    line.textContent = message;
    logBox.append(line);
    scrollToBottom();
}

/** 等這一輪版面算完再捲，否則長訊息換行後的高度還沒進 scrollHeight，會捲不到底。 */
function scrollToBottom(): void {
    requestAnimationFrame(() => {
        logBox.scrollTop = logBox.scrollHeight;
    });
}

function setProgress(ratio: number): void {
    bar.style.width = `${Math.round(Math.max(0, Math.min(1, ratio)) * 100)}%`;
}

function setRunning(next: boolean): void {
    running = next;
    startBtn.disabled = next || !dirInput.value;
    browseBtn.disabled = next;
    startBtn.textContent = next ? '執行中…' : '開始';
}

async function chooseDir(): Promise<void> {
    const picked = await open({
        directory: true,
        multiple: false,
        title: '選擇《大富翁2》的遊戲目錄',
        defaultPath: dirInput.value || undefined,
    });
    if (typeof picked === 'string') {
        dirInput.value = picked;
        dirInput.title = picked;   // 路徑太長被截斷時，滑鼠停留可看全文
        setRunning(false);
    }
}

async function start(): Promise<void> {
    if (running || !dirInput.value) return;

    clearLog();
    setProgress(0);
    setRunning(true);

    try {
        const summary = await invoke<string>('run_patch', { targetDir: dirInput.value });
        setProgress(1);
        appendSummary('');
        summary.split('\n').forEach(appendSummary);
    } catch (err) {
        // Rust 端已經送過 FATAL 事件了，這裡只確保使用者一定看得到
        appendLog('FATAL', String(err));
    } finally {
        setRunning(false);
    }
}

// Rust 送來的每一則日誌
await listen<LogEvent>('patch://log', ({ payload }) => {
    appendLog(payload.level, payload.message);
    if (payload.step !== null && payload.total > 0) {
        setProgress(payload.step / payload.total);
    }
});

browseBtn.addEventListener('click', () => {
    void chooseDir();
});
startBtn.addEventListener('click', () => {
    void start();
});

// 沿用 Python 版的貼心設計：預設帶入程式所在目錄，讓「丟進遊戲資料夾直接執行」也能用。
// 由 Rust 端提供，不需要給前端任何檔案系統權限。
try {
    const cwd = await invoke<string>('default_dir');
    if (cwd) {
        dirInput.value = cwd;
        dirInput.title = cwd;
    }
} catch {
    // 取不到就讓使用者自己挑，不是錯誤
}

setRunning(false);
