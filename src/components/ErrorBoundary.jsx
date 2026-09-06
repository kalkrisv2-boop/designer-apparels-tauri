import React from "react";

export default class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { error: null, info: null };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  componentDidCatch(error, info) {
    this.setState({ info });
    // Still log to devtools console for full stack trace
    console.error("Render error caught by ErrorBoundary:", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 24, fontFamily: "monospace", color: "#8a2c22" }}>
          <h2>Something went wrong</h2>
          <p>
            The app hit an error while rendering this screen. Copy the details
            below back to support so it can be fixed — the rest of the app
            (other tabs) should still work if you restart.
          </p>
          <pre
            style={{
              whiteSpace: "pre-wrap",
              background: "#fdecea",
              border: "1px solid #f5c2be",
              padding: 12,
              borderRadius: 6,
              fontSize: 12,
            }}
          >
            {String(this.state.error && this.state.error.stack || this.state.error)}
            {this.state.info ? "\n\nComponent stack:" + this.state.info.componentStack : ""}
          </pre>
        </div>
      );
    }
    return this.props.children;
  }
}
