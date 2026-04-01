export default function NotFound() {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '60vh',
        gap: '1rem',
        textAlign: 'center',
      }}
    >
      <h1 style={{ fontSize: '3rem', fontWeight: 800, opacity: 0.2 }}>404</h1>
      <p style={{ opacity: 0.6 }}>This page could not be found.</p>
      <a
        href="/"
        style={{
          padding: '0.5rem 1rem',
          borderRadius: '8px',
          border: '1px solid rgba(128,128,128,0.2)',
          textDecoration: 'none',
          fontSize: '0.875rem',
        }}
      >
        Back to docs
      </a>
    </div>
  )
}
