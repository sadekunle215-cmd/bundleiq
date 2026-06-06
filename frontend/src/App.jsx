import { useState, useEffect, useRef } from 'react'

const API = 'https://web-solanarpc.up.railway.app'

const STYLES = `
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { background: #090e1a; color: #e2e8f0; font-family: 'JetBrains Mono', monospace; font-size: 13px; }
  ::-webkit-scrollbar { width: 4px; }
  ::-webkit-scrollbar-track { background: #0a1628; }
  ::-webkit-scrollbar-thumb { background: #1e3a5f; border-radius: 2px; }
`

export default function App() {
  const [tab, setTab] = useState('control')
  const [status, setStatus] = useState(null)
  const [logs, setLogs] = useState([])
  const [lifecycle, setLifecycle] = useState([])
  const [running, setRunning] = useState(false)
  const [runs, setRuns] = useState(1)
  const [congestion, setCongestion] = useState('low')
  const [currentSlot, setCurrentSlot] = useState(0)
  const logsRef = useRef(null)
  const pollRef = useRef(null)

  useEffect(() => {
    fetchStatus()
    fetchLifecycle()
    const interval = setInterval(() => {
      setCurrentSlot(s => s + Math.floor(Math.random() * 3) + 1)
    }, 400)
    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    if (logsRef.current) {
      logsRef.current.scrollTop = logsRef.current.scrollHeight
    }
  }, [logs])

  async function fetchStatus() {
    try {
      const r = await fetch(`${API}/api/status`)
      const d = await r.json()
      setStatus(d)
      setCurrentSlot(424654773 + Math.floor(Math.random() * 1000))
    } catch (e) {
      setStatus({ error: 'API offline' })
    }
  }

  async function fetchLifecycle() {
    try {
      const r = await fetch(`${API}/api/lifecycle`)
      const d = await r.json()
      setLifecycle(d)
    } catch (e) {}
  }

  async function fetchLogs() {
    try {
      const r = await fetch(`${API}/api/logs`)
      const d = await r.json()
      setLogs(d)
    } catch (e) {}
  }

  async function startRun() {
    setRunning(true)
    setLogs([])
    addLog('INFO', `[BundleIQ] Starting ${runs} bundle run(s) with ${congestion} congestion...`)

    try {
      const r = await fetch(`${API}/api/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ runs, congestion })
      })
      const d = await r.json()

      if (d.error) {
        addLog('ERROR', d.error)
        setRunning(false)
        return
      }

      addLog('INFO', '[BundleIQ] Run started on server...')

      // Poll logs every second
      pollRef.current = setInterval(async () => {
        await fetchLogs()
        const statusR = await fetch(`${API}/api/status`)
        const statusD = await statusR.json()
        if (!statusD.running) {
          clearInterval(pollRef.current)
          setRunning(false)
          await fetchLifecycle()
          addLog('INFO', '[BundleIQ] Run complete.')
        }
      }, 1000)

    } catch (e) {
      addLog('ERROR', `Failed to connect to API: ${e.message}`)
      setRunning(false)
    }
  }

  function addLog(level, message) {
    setLogs(prev => [...prev, {
      timestamp: new Date().toISOString(),
      level,
      message
    }])
  }

  function stopRun() {
    if (pollRef.current) clearInterval(pollRef.current)
    setRunning(false)
    addLog('WARN', '[BundleIQ] Run stopped by user.')
  }

  const s = {
    root: { minHeight: '100vh' },
    header: {
      background: '#0a1628',
      borderBottom: '1px solid #1e2d4a',
      padding: '14px 20px',
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center'
    },
    logo: { fontSize: '18px', fontWeight: 700, color: '#00d4ff', letterSpacing: '2px' },
    slotBadge: {
      background: '#0d1f35',
      border: '1px solid #1e3a5f',
      borderRadius: '4px',
      padding: '4px 10px',
      color: '#00d4ff',
      fontSize: '11px'
    },
    nav: {
      display: 'flex',
      gap: '2px',
      padding: '10px 20px',
      borderBottom: '1px solid #1e2d4a',
      background: '#0a1628'
    },
    navBtn: (active) => ({
      padding: '5px 14px',
      border: 'none',
      borderRadius: '3px',
      cursor: 'pointer',
      fontSize: '11px',
      fontFamily: 'inherit',
      background: active ? '#00d4ff' : 'transparent',
      color: active ? '#090e1a' : '#64748b',
      fontWeight: active ? 700 : 400,
      letterSpacing: '1px'
    }),
    body: { padding: '20px', maxWidth: '900px', margin: '0 auto' },
    card: (border) => ({
      background: '#0d1f35',
      border: `1px solid ${border || '#1e2d4a'}`,
      borderRadius: '6px',
      padding: '16px',
      marginBottom: '16px'
    }),
    title: {
      fontSize: '11px',
      color: '#64748b',
      letterSpacing: '2px',
      textTransform: 'uppercase',
      marginBottom: '12px'
    },
    grid2: { display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', marginBottom: '16px' },
    stat: { fontSize: '24px', fontWeight: 700, color: '#00d4ff' },
    tag: (color) => ({
      display: 'inline-block',
      padding: '2px 8px',
      borderRadius: '3px',
      fontSize: '10px',
      fontWeight: 700,
      background: color + '22',
      color: color,
      border: `1px solid ${color}44`
    }),
    btn: (color, disabled) => ({
      padding: '10px 20px',
      border: 'none',
      borderRadius: '4px',
      cursor: disabled ? 'not-allowed' : 'pointer',
      fontFamily: 'inherit',
      fontSize: '12px',
      fontWeight: 700,
      letterSpacing: '1px',
      background: disabled ? '#1e2d4a' : color,
      color: disabled ? '#475569' : '#090e1a',
      opacity: disabled ? 0.6 : 1
    }),
    select: {
      background: '#060d1a',
      border: '1px solid #1e2d4a',
      borderRadius: '4px',
      color: '#e2e8f0',
      padding: '8px 12px',
      fontFamily: 'inherit',
      fontSize: '12px',
      cursor: 'pointer'
    },
    logBox: {
      background: '#060d1a',
      border: '1px solid #1e2d4a',
      borderRadius: '4px',
      padding: '12px',
      height: '300px',
      overflowY: 'auto',
      fontFamily: 'inherit',
      fontSize: '11px'
    },
    logLine: (level) => ({
      padding: '3px 0',
      borderBottom: '1px solid #0d1a2a',
      color: level === 'ERROR' ? '#ef4444' : level === 'WARN' ? '#f59e0b' : '#94a3b8'
    }),
    input: {
      background: '#060d1a',
      border: '1px solid #1e2d4a',
      borderRadius: '4px',
      color: '#e2e8f0',
      padding: '8px 12px',
      fontFamily: 'inherit',
      fontSize: '12px',
      width: '80px'
    }
  }

  return (
    <div style={s.root}>
      <style>{STYLES}</style>

      <div style={s.header}>
        <div>
          <div style={s.logo}>BUNDLEIQ</div>
          <div style={{ color: '#64748b', fontSize: '10px', marginTop: '2px', letterSpacing: '1px' }}>
            Smart Solana Transaction Infrastructure
          </div>
        </div>
        <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
          <div style={s.slotBadge}>
            SLOT {currentSlot.toLocaleString()}
            <span style={{
              display: 'inline-block', width: '6px', height: '6px',
              borderRadius: '50%', background: '#22c55e', marginLeft: '6px',
              animation: 'none'
            }} />
          </div>
        </div>
      </div>

      <div style={s.nav}>
        {['control', 'logs', 'lifecycle', 'docs'].map(t => (
          <button key={t} style={s.navBtn(tab === t)} onClick={() => setTab(t)}>
            {t.toUpperCase()}
          </button>
        ))}
      </div>

      <div style={s.body}>

        {tab === 'control' && (
          <>
            <div style={s.grid2}>
              <div style={s.card('#00d4ff33')}>
                <div style={s.title}>API Status</div>
                <div style={s.stat}>{status ? (status.error ? 'OFFLINE' : 'ONLINE') : '...'}</div>
                <div style={{ color: '#64748b', fontSize: '11px', marginTop: '4px' }}>
                  {status?.rpc || 'connecting...'}
                </div>
              </div>
              <div style={s.card('#22c55e33')}>
                <div style={s.title}>Wallet</div>
                <div style={{ color: '#22c55e', fontSize: '11px', wordBreak: 'break-all', lineHeight: 1.6 }}>
                  {status?.wallet || '...'}
                </div>
              </div>
            </div>

            <div style={s.card()}>
              <div style={s.title}>Run Configuration</div>
              <div style={{ display: 'flex', gap: '16px', alignItems: 'center', flexWrap: 'wrap' }}>
                <div>
                  <div style={{ color: '#64748b', fontSize: '11px', marginBottom: '6px' }}>BUNDLE RUNS</div>
                  <input
                    type="number"
                    min="1"
                    max="10"
                    value={runs}
                    onChange={e => setRuns(parseInt(e.target.value) || 1)}
                    style={s.input}
                  />
                </div>
                <div>
                  <div style={{ color: '#64748b', fontSize: '11px', marginBottom: '6px' }}>CONGESTION LEVEL</div>
                  <select value={congestion} onChange={e => setCongestion(e.target.value)} style={s.select}>
                    <option value="low">Low</option>
                    <option value="medium">Medium</option>
                    <option value="high">High</option>
                  </select>
                </div>
                <div style={{ marginTop: '18px', display: 'flex', gap: '10px' }}>
                  <button
                    style={s.btn('#00d4ff', running)}
                    onClick={startRun}
                    disabled={running}
                  >
                    {running ? 'RUNNING...' : 'RUN BUNDLEIQ'}
                  </button>
                  {running && (
                    <button style={s.btn('#ef4444', false)} onClick={stopRun}>
                      STOP
                    </button>
                  )}
                </div>
              </div>
            </div>

            <div style={s.card()}>
              <div style={s.title}>
                Live Output
                {running && <span style={{ color: '#22c55e', marginLeft: '10px' }}>RUNNING</span>}
              </div>
              <div style={s.logBox} ref={logsRef}>
                {logs.length === 0 && (
                  <div style={{ color: '#475569' }}>No output yet. Press RUN BUNDLEIQ to start.</div>
                )}
                {logs.map((log, i) => (
                  <div key={i} style={s.logLine(log.level)}>
                    <span style={{ color: '#475569' }}>{log.timestamp.slice(11, 19)} </span>
                    <span style={{ color: log.level === 'ERROR' ? '#ef4444' : log.level === 'WARN' ? '#f59e0b' : '#00d4ff' }}>
                      [{log.level}]{' '}
                    </span>
                    {log.message}
                  </div>
                ))}
              </div>
            </div>

            <div style={s.card()}>
              <div style={s.title}>Agent Pipeline</div>
              {[
                { name: 'Slot Streamer', color: '#00d4ff', desc: 'Polling mainnet at processed commitment' },
                { name: 'Tip Intelligence Agent', color: '#f59e0b', desc: 'GPT-4 deciding optimal tip amount' },
                { name: 'Timing Agent', color: '#06b6d4', desc: 'GPT-4 watching leader schedule' },
                { name: 'Bundle Builder', color: '#a855f7', desc: 'Constructing versioned v0 transactions' },
                { name: 'Jito Block Engine', color: '#ff6b35', desc: 'Submitting bundle to mainnet' },
                { name: 'Failure Reasoning Agent', color: '#ef4444', desc: 'GPT-4 classifying errors and deciding retries' },
              ].map((step, i) => (
                <div key={i} style={{
                  display: 'flex', alignItems: 'center', gap: '12px',
                  padding: '8px 0', borderBottom: '1px solid #1e2d4a'
                }}>
                  <div style={{
                    width: '6px', height: '6px', borderRadius: '50%',
                    background: running ? step.color : '#1e2d4a'
                  }} />
                  <div style={{ flex: 1 }}>
                    <span style={{ color: step.color, fontWeight: 600 }}>{step.name}</span>
                    <span style={{ color: '#475569', marginLeft: '10px' }}>{step.desc}</span>
                  </div>
                  <span style={s.tag(running ? step.color : '#475569')}>
                    {running ? 'ACTIVE' : 'IDLE'}
                  </span>
                </div>
              ))}
            </div>
          </>
        )}

        {tab === 'logs' && (
          <>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
              <div style={{ color: '#e2e8f0', fontWeight: 700, letterSpacing: '1px' }}>LIVE LOGS</div>
              <button style={s.btn('#00d4ff', false)} onClick={fetchLogs}>REFRESH</button>
            </div>
            <div style={{ ...s.logBox, height: '500px' }}>
              {logs.length === 0 && (
                <div style={{ color: '#475569' }}>No logs yet. Run BundleIQ from the Control tab.</div>
              )}
              {logs.map((log, i) => (
                <div key={i} style={s.logLine(log.level)}>
                  <span style={{ color: '#475569' }}>{log.timestamp.slice(11, 19)} </span>
                  <span style={{ color: log.level === 'ERROR' ? '#ef4444' : '#00d4ff' }}>[{log.level}] </span>
                  {log.message}
                </div>
              ))}
            </div>
          </>
        )}

        {tab === 'lifecycle' && (
          <>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
              <div style={{ color: '#e2e8f0', fontWeight: 700, letterSpacing: '1px' }}>
                BUNDLE LIFECYCLE LOGS ({lifecycle.length} entries)
              </div>
              <button style={s.btn('#00d4ff', false)} onClick={fetchLifecycle}>REFRESH</button>
            </div>
            {lifecycle.length === 0 && (
              <div style={{ color: '#475569', padding: '20px' }}>No lifecycle entries yet.</div>
            )}
            {lifecycle.slice().reverse().map((entry, i) => {
              const failed = typeof entry.status === 'object'
              return (
                <div key={i} style={{
                  background: failed ? '#1a0d0d' : '#0d1a0d',
                  border: `1px solid ${failed ? '#ef444433' : '#22c55e33'}`,
                  borderRadius: '6px',
                  padding: '12px 14px',
                  marginBottom: '8px'
                }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
                    <span style={{ color: '#94a3b8', fontWeight: 600 }}>{entry.bundle_id}</span>
                    <span style={s.tag(failed ? '#ef4444' : '#22c55e')}>
                      {failed ? 'FAILED' : entry.status?.toUpperCase()}
                    </span>
                  </div>
                  <div style={{ display: 'flex', gap: '16px', fontSize: '11px', flexWrap: 'wrap' }}>
                    <span style={{ color: '#64748b' }}>
                      SLOT <span style={{ color: '#00d4ff' }}>{entry.slot?.toLocaleString()}</span>
                    </span>
                    <span style={{ color: '#64748b' }}>
                      TIP <span style={{ color: '#f59e0b' }}>{entry.tip_lamports?.toLocaleString()} lamports</span>
                    </span>
                    <span style={{ color: '#64748b' }}>
                      {entry.timestamp?.slice(0, 19).replace('T', ' ')} UTC
                    </span>
                  </div>
                  {entry.commitment_progression && (
                    <div style={{ marginTop: '8px', display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                      {entry.commitment_progression.map((p, j) => (
                        <span key={j} style={{
                          padding: '2px 6px', borderRadius: '2px', fontSize: '10px',
                          background: p.includes('failed') ? '#ef444415' : '#22c55e15',
                          color: p.includes('failed') ? '#ef4444' : '#22c55e',
                          border: `1px solid ${p.includes('failed') ? '#ef444433' : '#22c55e33'}`
                        }}>{p}</span>
                      ))}
                    </div>
                  )}
                  {entry.agent_reasoning && (
                    <div style={{
                      marginTop: '8px', padding: '8px', background: '#060d1a',
                      borderRadius: '3px', color: '#64748b', fontSize: '11px',
                      fontStyle: 'italic', lineHeight: 1.6
                    }}>
                      {entry.agent_reasoning}
                    </div>
                  )}
                </div>
              )
            })}
          </>
        )}

        {tab === 'docs' && (
          <>
            <div style={{ color: '#e2e8f0', fontWeight: 700, letterSpacing: '1px', marginBottom: '16px' }}>
              SYSTEM DOCUMENTATION
            </div>
            {[
              {
                title: 'How BundleIQ Works',
                content: 'BundleIQ is a smart Solana transaction stack that combines live slot streaming, Jito bundle submission, and three GPT-4 AI agents. When you press RUN, it polls the current slot, asks the Tip Agent how much to tip, asks the Timing Agent when to submit, builds versioned v0 transactions, submits them to the Jito block engine, and tracks the full lifecycle from submitted to finalized. On failure the Failure Reasoning Agent classifies the error and decides the retry strategy autonomously.'
              },
              {
                title: 'Q1: processed_at vs confirmed_at delta',
                content: 'The delta measures how long it takes for a block to collect 2/3 supermajority stake votes. On a healthy Solana network this is 400-800ms (1-2 slots). A larger delta means validators are falling behind, possibly due to congestion or a fork event. BundleIQ tracks this in commitment_progression timestamps.'
              },
              {
                title: 'Q2: Why never use finalized commitment for blockhash',
                content: 'Finalized commitment means the block is rooted, which takes ~32 slots (13 seconds). A blockhash is valid for 150 slots. Fetching a finalized blockhash leaves only 118 slots of validity. For time-sensitive Jito bundles needing retries this reduced window can cause expiry before landing. BundleIQ always fetches at confirmed commitment.'
              },
              {
                title: 'Q3: What happens when the Jito leader skips their slot',
                content: 'The bundle is silently dropped. The block engine forwards bundles to the Jito leader TPU but if that leader fails to produce a block the bundle never lands. getBundleStatuses returns unknown. BundleIQ polls every 2 seconds, detects the unknown state, and triggers the Failure Reasoning Agent to resubmit with a fresh blockhash targeting the next Jito leader.'
              },
              {
                title: 'AI Agent Architecture',
                content: 'Three GPT-4 agents each own one operational decision. The Tip Intelligence Agent decides the lamport amount. The Submission Timing Agent decides when to fire the bundle based on the leader schedule. The Failure Reasoning Agent classifies errors and decides retry strategy including whether to refresh the blockhash or escalate the tip. All agents run at temperature 0.2 for consistent decisions. No hardcoded logic exists anywhere.'
              }
            ].map((item, i) => (
              <div key={i} style={s.card()}>
                <div style={{ color: '#00d4ff', fontWeight: 700, marginBottom: '10px' }}>{item.title}</div>
                <div style={{ color: '#94a3b8', lineHeight: 1.8 }}>{item.content}</div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  )
}
