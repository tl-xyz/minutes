import { DemoPlayer } from "@/components/demo-player";
import { CopyButton } from "@/components/copy-button";

export default function Home() {
  return (
    <div className="max-w-[800px] mx-auto px-6">
      {/* Nav */}
      <nav className="flex items-center justify-between py-5 border-b border-white/[0.06]">
        <div className="font-mono text-[15px] font-medium text-[#ededed]">
          minutes
        </div>
        <div className="flex gap-6 text-sm">
          <a href="https://github.com/silverstein/minutes" className="text-[#666] hover:text-[#ededed] transition-colors">GitHub</a>
          <a href="https://github.com/silverstein/minutes#install" className="text-[#666] hover:text-[#ededed] transition-colors">Install</a>
          <a href="https://github.com/silverstein/minutes#claude-integration" className="text-[#666] hover:text-[#ededed] transition-colors">Claude</a>
          <a href="/llms.txt" className="text-[#666] hover:text-[#ededed] transition-colors">llms.txt</a>
        </div>
      </nav>

      {/* Hero */}
      <section className="relative pt-20 pb-14 text-center">
        {/* Subtle radial glow */}
        <div className="absolute -top-[40%] left-1/2 -translate-x-1/2 w-[600px] h-[600px] bg-[radial-gradient(circle,rgba(0,112,243,0.08)_0%,rgba(168,85,247,0.04)_40%,transparent_70%)] pointer-events-none" />

        <h1 className="relative text-[44px] font-bold leading-[1.15] mb-4 tracking-[-0.04em] bg-gradient-to-b from-white to-[#a1a1a1] bg-clip-text text-transparent">
          Your AI remembers every<br />conversation you&apos;ve had
        </h1>
        <p className="relative text-[17px] text-[#a1a1a1] max-w-[520px] mx-auto mb-10 leading-relaxed">
          Record meetings. Capture voice memos. Ask Claude what was decided three weeks ago. Everything runs locally. Open source, free forever.
        </p>

        {/* Remotion Player */}
        <div className="relative mb-10">
          <DemoPlayer />
        </div>

        {/* Install commands */}
        <div className="flex gap-3 justify-center flex-wrap mb-3">
          <CopyButton label="Desktop app" cmd="brew install --cask silverstein/tap/minutes" />
          <CopyButton label="CLI only" cmd="brew tap silverstein/tap && brew install minutes" />
          <CopyButton label="MCP server" cmd="npx minutes-mcp" />
        </div>
        <p className="text-[13px] text-[#666]">
          macOS, Windows, Linux. <code className="font-mono text-[12px] text-[#a1a1a1]">npx</code> works everywhere — Claude Desktop, Cursor, Windsurf, any MCP client.
        </p>
      </section>

      {/* How it works */}
      <section className="py-14 border-t border-white/[0.06]">
        <h2 className="text-2xl font-semibold mb-6 tracking-[-0.03em]">How it works</h2>
        <pre className="font-mono text-[13px] leading-relaxed text-[#a1a1a1] bg-[#0a0a0a] border border-white/[0.06] rounded-lg p-5 overflow-x-auto mb-4">
{`Audio  →  Transcribe  →  Summarize  →  Structured Markdown
          (local)        (your LLM)     (decisions, action items,
         whisper.cpp    Claude / Ollama   people, entities)`}
        </pre>
        <p className="text-sm text-[#a1a1a1] leading-relaxed">
          Your audio never leaves your machine. Transcription is local via whisper.cpp with GPU acceleration. Summarization is optional — Claude does it conversationally when you ask, using your existing subscription. No API keys needed.
        </p>
      </section>

      {/* Audiences */}
      <section className="py-14 border-t border-white/[0.06]">
        <h2 className="text-2xl font-semibold mb-6 tracking-[-0.03em]">Built for everyone who has conversations</h2>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
          {[
            {
              title: "Developers",
              desc: "15 CLI commands. 136 tests. Rust engine, single binary, MIT license. Homebrew, cross-platform CI. TypeScript SDK for agent developers.",
            },
            {
              title: "Knowledge workers",
              desc: "Menu bar app with one-click recording. Calendar integration suggests recording before meetings. Voice memo pipeline from iPhone. Obsidian vault sync.",
            },
            {
              title: "AI agents",
              desc: "13 MCP tools. 7 resources. Structured intents in YAML. Decision consistency tracking. People profiles. Any agent that speaks MCP can use Minutes as its memory layer.",
            },
          ].map((card) => (
            <div
              key={card.title}
              className="p-5 bg-[#0a0a0a] border border-white/[0.06] rounded-lg transition-colors hover:border-white/[0.12]"
            >
              <h3 className="text-[15px] font-semibold mb-2">{card.title}</h3>
              <p className="text-[13px] text-[#a1a1a1] leading-snug">{card.desc}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Features */}
      <section className="py-14 border-t border-white/[0.06]">
        <h2 className="text-2xl font-semibold mb-6 tracking-[-0.03em]">What you get</h2>
        <div className="grid gap-4">
          {[
            ["Local transcription", "whisper.cpp with GPU acceleration (Metal, CUDA, CoreML). Your audio never leaves your machine."],
            ["Streaming transcription", "Text appears as you speak. Partial results every 2 seconds, final transcript when you stop."],
            ["Dictation mode", "Hold a hotkey, speak, release. Text goes to clipboard and daily note. Works from the menu bar app or CLI."],
            ["Speaker diarization", "pyannote separates \"who said what\" in multi-person meetings."],
            ["Structured extraction", "Action items, decisions, and commitments as queryable YAML, not buried in prose."],
            ["Cross-meeting intelligence", "Search across all meetings. Build people profiles from every conversation."],
            ["Voice memo pipeline", "iPhone Voice Memos → iCloud → auto-transcribe on Mac. Ideas while walking the dog, searchable by afternoon."],
            ["Desktop app", "Tauri v2 menu bar app. One-click recording, dictation hotkey, calendar integration. macOS and Windows."],
            ["Claude-native", "13 MCP tools for Claude Desktop, Cowork, Dispatch. Claude Code plugin with 12 skills. No API keys."],
            ["Any LLM", "Ollama for local. OpenAI if you prefer. Or skip summarization entirely — the transcript is the artifact."],
            ["Markdown is the truth", "Every meeting saves as markdown with YAML frontmatter. Works with Obsidian, grep, QMD, or anything."],
          ].map(([title, desc]) => (
            <div key={title} className="flex gap-3 items-start text-sm">
              <span className="text-[#666] font-mono text-[13px] mt-0.5 shrink-0">&gt;</span>
              <p className="text-[#a1a1a1] leading-snug">
                <strong className="text-[#ededed] font-medium">{title}</strong> — {desc}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Comparison */}
      <section className="py-14 border-t border-white/[0.06]">
        <h2 className="text-2xl font-semibold mb-6 tracking-[-0.03em]">How it compares</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-[13px] border-collapse">
            <thead>
              <tr>
                <th className="text-left p-2.5 border-b border-white/[0.06] text-[#666] font-medium text-xs uppercase tracking-wider" />
                <th className="text-left p-2.5 border-b border-white/[0.06] text-[#666] font-medium text-xs uppercase tracking-wider">Granola</th>
                <th className="text-left p-2.5 border-b border-white/[0.06] text-[#666] font-medium text-xs uppercase tracking-wider">Otter.ai</th>
                <th className="text-left p-2.5 border-b border-white/[0.06] text-[#666] font-medium text-xs uppercase tracking-wider">Meetily</th>
                <th className="text-left p-2.5 border-b border-white/[0.06] text-[#ededed] font-semibold text-xs uppercase tracking-wider">minutes</th>
              </tr>
            </thead>
            <tbody>
              {([
                ["Local transcription", "No", "No", "Yes", "Yes"],
                ["Open source", "No", "No", "Yes", "MIT"],
                ["Free", "$18/mo", "Freemium", "Free", "Free"],
                ["AI agent integration", "No", "No", "No", "13 MCP tools"],
                ["Cross-meeting intelligence", "No", "No", "No", "Yes"],
                ["Dictation mode", "No", "No", "No", "Yes"],
                ["Voice memos", "No", "No", "No", "iPhone pipeline"],
                ["People memory", "No", "No", "No", "Yes"],
                ["Data ownership", "Their servers", "Their servers", "Local", "Local"],
              ] as const).map(([feature, ...values]) => (
                <tr key={feature}>
                  <td className="p-2.5 border-b border-white/[0.03] text-[#ededed] font-medium">{feature}</td>
                  {values.map((val, i) => {
                    const isMinutes = i === 3;
                    const isYes = val === "Yes" || val === "Local" || val === "Free";
                    const isNo = val === "No";
                    return (
                      <td
                        key={i}
                        className={`p-2.5 border-b border-white/[0.03] ${
                          isMinutes
                            ? "text-[#ededed] font-semibold"
                            : isYes
                              ? "text-[#00cc88]"
                              : isNo
                                ? "text-[#666]"
                                : "text-[#a1a1a1]"
                        }`}
                      >
                        {val}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-12 border-t border-white/[0.06] text-center text-[13px] text-[#666]">
        <p>minutes is MIT licensed and free forever. Built with Rust, whisper.cpp, and Tauri.</p>
        <p className="mt-2">
          <a href="https://github.com/silverstein/minutes" className="text-[#666] hover:text-[#a1a1a1] transition-colors">GitHub</a>
          {" · "}
          <a href="/llms.txt" className="text-[#666] hover:text-[#a1a1a1] transition-colors">llms.txt</a>
          {" · "}
          <a href="https://github.com/silverstein/minutes/blob/main/CONTRIBUTING.md" className="text-[#666] hover:text-[#a1a1a1] transition-colors">Contribute</a>
        </p>
      </footer>
    </div>
  );
}
