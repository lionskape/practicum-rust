import { useEffect, useState } from "react";

export default function Index() {
  const [ClientTerminal, setClientTerminal] = useState<React.ComponentType | null>(null);

  useEffect(() => {
    import("../components/TerminalApp").then((m) => setClientTerminal(() => m.default));
  }, []);

  if (!ClientTerminal) {
    return (
      <main className="page" style={{ padding: "2rem" }}>
        <div className="terminal">
          <div className="terminal-header">
            <span className="terminal-title">mini-launchpad@localnet</span>
          </div>
          <pre className="terminal-body">
            <span className="term-line term-muted">загрузка...</span>
          </pre>
        </div>
      </main>
    );
  }

  return (
    <main className="page" style={{ padding: "2rem" }}>
      <ClientTerminal />
    </main>
  );
}
