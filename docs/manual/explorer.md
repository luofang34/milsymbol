# Symbol lab

Choose a category and an entity, or enter a SIDC. The preview, code meanings,
validation, and SVG come from this checkout's Rust library compiled to WASM.
The page runs locally in your browser; no input is sent to a server.

<div id="symbol-lab-app" class="symbol-lab" aria-busy="true">
  <p id="lab-status" role="status" aria-live="polite">Loading the Rust renderer…</p>
  <div id="lab-presets" class="lab-presets" aria-label="Examples"></div>
  <div class="lab-sidc-row">
    <label for="lab-sidc">SIDC <span>20 or 30 numeric characters, or a legacy letter code</span></label>
    <div><input id="lab-sidc" type="text" spellcheck="false" autocomplete="off"><button id="lab-apply" type="button">Interpret SIDC</button></div>
  </div>
  <div class="lab-layout">
    <form id="lab-controls" class="lab-controls">
      <fieldset><legend>Meaning and display</legend>
        <label>Code edition <select id="lab-version" data-field="version"><option value="15">15 · MIL 2525E Change 1</option><option value="16">16 · APP-6(E) Version 2</option><option value="13">13 · MIL 2525E</option><option value="14">14 · APP-6(E) Version 1</option><option value="10">10 · Edition D</option><option value="11">11 · MIL 2525D(1)</option><option value="12">12 · APP-6(D)(2)</option></select></label>
        <label>Display standard <select id="lab-standard"><option value="Mil2525">MIL-STD-2525</option><option value="App6">NATO APP-6</option></select></label>
        <label class="lab-wide">Rendering rules <select id="lab-mode"><option value="Standards">Pinned standards</option><option value="Milsymbol304">milsymbol.js 3.0.4 compatibility</option></select></label>
      </fieldset>
      <fieldset id="lab-numeric"><legend>Build a numeric SIDC</legend>
        <label class="lab-wide">Category · digits 5–6 <select id="lab-symbol_set" data-field="symbol_set"></select></label>
        <label class="lab-wide">Find an entity <input id="lab-search" type="search" placeholder="Name or code in this category" autocomplete="off"></label>
        <label class="lab-wide">Entity · digits 11–16 <select id="lab-entity" data-field="entity"></select></label>
        <label>Context · digit 3 <select id="lab-context" data-field="context"></select></label>
        <label>Identity · digit 4 <select id="lab-identity" data-field="identity"></select></label>
        <label>Status · digit 7 <select id="lab-status-code" data-field="status"></select></label>
        <label>HQ / task force / dummy · digit 8 <select id="lab-hq" data-field="hq"></select></label>
        <label class="lab-wide">Amplifier · digits 9–10 <select id="lab-amplifier" data-field="amplifier"></select></label>
        <label class="lab-wide">Sector 1 modifier <select id="lab-modifier1" data-field="modifier1"></select></label>
        <label class="lab-wide">Sector 2 modifier <select id="lab-modifier2" data-field="modifier2"></select></label>
      </fieldset>
      <fieldset><legend>Style and input</legend>
        <label>Size <input id="lab-size" type="number" min="10" max="300" step="5"></label>
        <label>Direction (degrees) <input id="lab-direction" type="number" step="1" placeholder="None"></label>
        <label>Unique designation <input id="lab-label" type="text" placeholder="e.g. 1–66"></label>
        <label id="lab-graphic-label">Graphic input <input id="lab-graphic-value" type="text" placeholder="Height, depth or pressure"></label>
        <div class="lab-wide lab-checks"><label><input id="lab-fill" type="checkbox"> Fill</label><label><input id="lab-frame" type="checkbox"> Frame</label><label><input id="lab-icon" type="checkbox"> Icon</label><label><input id="lab-guides" type="checkbox"> Bounds and anchors</label></div>
      </fieldset>
    </form>
    <div class="lab-result">
      <div class="lab-preview" id="lab-preview" aria-label="Rendered symbol"></div>
      <p class="lab-legend" id="lab-guide-legend" hidden>Dashed rectangle: geometry bounds · red cross: map anchor · blue circle: icon centre</p>
      <div class="lab-result-title"><strong id="lab-name"></strong><span id="lab-validity"></span></div>
      <p id="lab-explanation"></p>
      <p id="lab-citation" class="lab-citation"></p>
      <dl id="lab-metrics" class="lab-metrics"></dl>
      <div class="lab-actions"><button type="button" data-copy="sidc">Copy SIDC</button><button type="button" data-copy="svg">Copy SVG</button><button type="button" data-copy="config">Copy configuration</button><button type="button" data-copy="rust">Copy Rust</button></div>
      <details><summary>Native Rust call</summary><p class="lab-code-note">Requires <code>milsymbol</code> with its <code>serde</code> feature and <code>serde_json</code>.</p><pre><code id="lab-rust" class="language-rust"></code></pre></details>
      <details><summary>Interpreted metadata</summary><pre><code id="lab-metadata"></code></pre></details>
    </div>
  </div>
</div>
<noscript>The interactive lab requires JavaScript to load WASM. The native examples and generated SVGs below do not.</noscript>
<script type="module">import(`${path_to_root}assets/explorer.js`);</script>

The code edition controls what an entity or modifier means. Changing the display
standard changes frames and presentation; it does not reinterpret the entity
as a different edition. The selector excludes geographic lines and areas that
require multiple control points. Hierarchy entries deliberately have no icon.

For weather graphics, the input is the standard's numeric height, depth, or
pressure field. No unit conversion is performed. Tropopause height uses the
standard's chosen hundreds-of-feet or metres convention; pressure is in
hectopascals. The copied Rust call contains the exact input you supplied.

See [worked examples](examples.md) for compiled Rust source and generated output,
and [SIDCs and validation](sidcs.md) for strict error handling.
