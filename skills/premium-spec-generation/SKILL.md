---
name: premium-spec-generation
description: How to generate executable JSON specifications for Type 5 Premium infographics — the publication-quality artifacts that owner produces manually using Midjourney + Figma. The spec must be detailed enough that owner can execute without further questions. Reference quality: BMW iM3 blueprint, SR-71 Blackbird overview, Premium Cake Collection examples.
---

# Premium Spec Generation — Specifications for Manual Production

Type 5 visuals are not auto-generated. They are **executed by the owner** using Midjourney v7 (base image) + Figma (text overlays + assembly) following a detailed JSON specification.

This skill produces those specifications. The output is JSON, not SVG. The spec must be **executable** — owner reads it, follows it, produces publication-quality infographic in 1-2 hours.

## Prerequisites

- `visual-grammar` loaded (brand identity is reference, not strict — premium can use extended palette if justified)
- `compression-principle` loaded (premium still respects hierarchy)
- `memorable-design-techniques` loaded (premium leans heavily on these)
- Source identified with strong central theme

## Core principle

> A spec is a contract: if owner executes it faithfully, the result matches the design intent. The spec writer thinks like a creative director, not like a software engineer. Vague spec ("make it cool") = bad result. Detailed spec ("Midjourney prompt: X, then in Figma place headline at 50% width, 8% from top, font Inter Tight 700 64pt color #1a1a2e") = professional result.

## Reference quality benchmark

Compare to:

**BMW iM3 blueprint style** — engineering technical drawing with:
- Multiple views (front, rear, side, top, exploded)
- Dimensional callouts
- Component close-ups with descriptions
- Restrained color palette (cream + technical blue + accent color from object)
- Premium typography

**SR-71 Blackbird overview** — comprehensive technical infographic:
- Hero image of object
- Specifications panel
- Performance highlights
- Mission profile
- Comparative panels
- Historical timeline
- Dense but legible information layers

**Premium Cake Collection style** — catalog/showcase infographic:
- Multiple items rendered with photographic quality
- Consistent rendering style across items
- Annotations and label callouts
- Color palette derived from subject matter
- Magazine-quality typography

These are reference quality. Premium specs target this level.

## Spec structure

```json
{
  "version": "v1.0",
  "title": "Main title (max 80 chars)",
  "subtitle": "Subtitle/context (max 150 chars)",
  "format": "portrait | landscape | square",
  "dimensions": "1080x1350 | 1200x900 | 1080x1080",

  "midjourney_base_prompt": "Full Midjourney v7 prompt — describe scene/object/style/mood/composition in detail. Should be 50-100 words. Include style anchors like 'engineering blueprint style', 'magazine infographic style', 'cutaway technical illustration'.",

  "midjourney_aspect_ratio": "--ar 4:5 | --ar 16:9 | --ar 1:1",
  "midjourney_style_params": "--style raw --v 7 --stylize 100",
  "midjourney_negative_prompt": "--no text, watermark, signatures, busy background",

  "color_palette": {
    "primary": "#hexvalue",
    "secondary": "#hexvalue",
    "accent": "#hexvalue",
    "rationale": "why these colors for this subject"
  },

  "post_production_in_figma": {
    "canvas": {"width": 1080, "height": 1350},

    "text_overlays": [
      {
        "id": "headline",
        "type": "headline",
        "text": "Main Title Text",
        "position": {"x_percent": 50, "y_percent": 8},
        "anchor": "center",
        "font_family": "Inter Tight",
        "font_weight": 700,
        "font_size_pt": 64,
        "color": "#1a1a2e",
        "background": "none | white_pill | dark_pill",
        "letter_spacing": -0.02
      },
      {
        "id": "data_hero",
        "type": "data_number",
        "text": "65%",
        "position": {"x_percent": 25, "y_percent": 30},
        "anchor": "center",
        "font_family": "IBM Plex Mono",
        "font_weight": 500,
        "font_size_pt": 96,
        "color": "#c9a961",
        "caption_below": {
          "text": "probability of devaluation in 6 months",
          "font_size_pt": 14,
          "color": "#5a5a5a"
        }
      },
      // ... more text overlays
    ],

    "callouts_and_lines": [
      {
        "type": "leader_line",
        "from": {"x_percent": 30, "y_percent": 40},
        "to": {"x_percent": 50, "y_percent": 50},
        "style": {
          "stroke_color": "#5a5a5a",
          "stroke_width_pt": 1,
          "stroke_pattern": "solid | dashed"
        },
        "endpoint_marker": "none | dot | arrow",
        "label": {
          "text": "Engine intake cone",
          "position": "midpoint | start | end",
          "font_size_pt": 11
        }
      }
    ],

    "data_visualizations": [
      {
        "id": "scenario_bars",
        "type": "horizontal_bar_chart",
        "position": {"x_percent": 10, "y_percent": 70, "width_percent": 80, "height_percent": 20},
        "data": [
          {"label": "Devaluation 20-30%", "value": 65, "color": "#c9a961"},
          {"label": "Stable peg", "value": 20, "color": "#1a1a2e"},
          {"label": "Forced break", "value": 10, "color": "#8b2942"},
          {"label": "Devaluation >30%", "value": 5, "color": "#1a1a2e"}
        ],
        "axis_labels": false,
        "value_labels_position": "right_of_bar"
      }
    ],

    "decorative_elements": [
      {
        "type": "divider_line",
        "from": {"x_percent": 10, "y_percent": 15},
        "to": {"x_percent": 90, "y_percent": 15},
        "style": "thin_gold | thick_navy"
      }
    ]
  },

  "production_workflow": [
    "Step 1: Run Midjourney prompt. Generate 4 variants, pick best.",
    "Step 2: In Figma, import image at full canvas size.",
    "Step 3: Apply text overlays per specifications.",
    "Step 4: Add callouts and leader lines.",
    "Step 5: Add data visualizations as overlay shapes.",
    "Step 6: Export as PNG at 2x resolution for archive."
  ],

  "estimated_production_time_minutes": 90,

  "reference_examples": [
    {
      "name": "BMW iM3 blueprint",
      "applicable_aspect": "technical drawing style with callouts"
    },
    {
      "name": "SR-71 Blackbird overview",
      "applicable_aspect": "data-rich layout with photo + specs panels"
    }
  ],

  "quality_checklist": [
    "Headline readable at thumbnail size?",
    "Hero data number is the visual focal point?",
    "All callouts terminate precisely on targets?",
    "Color palette consistent across all elements?",
    "Typography hierarchy clear (size + weight)?",
    "Print test: prints legibly at A4?"
  ]
}
```

## Midjourney prompt craft

This is the critical part of the spec. A bad MJ prompt = bad base image = bad final infographic.

### Anatomy of effective MJ v7 prompt

```
[subject in detail], [style anchor], [composition], [color palette], [mood], [technical params]
```

**Subject in detail:** what is being depicted. Be specific.
- ❌ "fusion reactor"
- ✅ "compact tokamak reactor with high-temperature superconducting magnets, plasma chamber visible, technical cutaway view showing internal components"

**Style anchor:** the genre/style of infographic.
- "engineering blueprint style, technical drawing, isometric perspective"
- "magazine infographic illustration, premium editorial style"
- "exploded view technical diagram"
- "scientific cross-section illustration"
- "isometric vector infographic style"
- "premium product catalog photography style"

**Composition:** where things go.
- "central hero subject, surrounded by component close-ups"
- "side-view with proportional dimensions"
- "exploded view showing layered components"
- "centered composition with surrounding annotation space"

**Color palette:** explicit colors.
- "navy blue and gold accent palette, cream background"
- "muted technical drawing style with single accent color"

**Mood:** atmosphere.
- "professional, authoritative, magazine-quality"
- "scientific precision, premium publication"

**Technical params:** MJ-specific.
- `--ar 4:5` (portrait)
- `--v 7` (version 7)
- `--style raw` (less artistic interpretation)
- `--stylize 100-300` (control aesthetic intensity)
- `--no text, watermarks, signatures` (avoid auto-added text)

### Example good MJ prompt

For OSP Analysis on Belarus BYN-USD:

```
A premium magazine infographic visualization of a national currency under pressure, showing Belarusian ruble symbol prominently in center with subtle storm-cloud-like financial indicators surrounding it, isometric perspective with chart elements floating around the central subject, restrained navy and cream color palette with single gold accent on the currency symbol, sophisticated editorial illustration style similar to The Economist or Bloomberg Businessweek graphics, professional and authoritative mood, surrounding empty space for annotations, --ar 4:5 --v 7 --style raw --stylize 150 --no text, watermarks, signatures
```

### Anti-patterns in MJ prompts

- **Generic style anchors.** "Cool infographic style" — MJ doesn't know what that is.
- **Conflicting style anchors.** "Photorealistic + cartoon style" — confuses MJ.
- **Requesting text in image.** MJ generates illegible "text". Always add text in Figma post.
- **Overlong prompts.** >150 words dilute attention. 50-100 words is sweet spot.
- **Vague composition.** "Beautiful composition" — needs to be specific about layout.

## Figma execution specifications

After MJ produces base image, owner imports to Figma and adds:

### Text overlay precision

Every text element specified with:
- Exact position (% of canvas)
- Anchor point (center / left / right)
- Font (only from brand palette: Inter Tight, Inter, IBM Plex Mono)
- Size in points (Figma uses pt, conversion handled)
- Weight (specific weight, not "bold")
- Color (exact hex)
- Optional: background pill, letter spacing, line height

### Leader lines (callouts)

Connect labels to image features:
- Start point (% coords)
- End point (% coords)
- Stroke style (solid 1pt navy, dashed for secondary)
- Endpoint markers (dot at termination, arrow for direction)
- Label text and position along line

### Data visualizations as overlays

If chart needed, specify exact:
- Position and size
- Chart type
- Data values
- Bar colors per data point
- Label positions

This way Figma user can build chart with exact specifications, not guess.

## Production workflow steps

Spec includes step-by-step:

1. **MJ generation:** run prompt with parameters, generate 4 variants, owner picks best (or re-rolls if none match)
2. **Figma import:** drag image into Figma canvas at specified dimensions
3. **Text overlay pass:** add all text elements with precise positioning
4. **Annotation pass:** add leader lines and callouts
5. **Data viz pass:** add charts/diagrams as Figma shapes
6. **Polish pass:** kerning, spacing fine-tunes
7. **Export:** PNG at 2x resolution + SVG export of overlays (for archive)
8. **Upload:** save to drive with tag `#визуал_premium`

## Anti-patterns

- **Vague specs.** "Make a nice headline" → can't execute. "Headline at 50%/8% center anchor Inter Tight 700 64pt #1a1a2e" → executable.
- **Conflicting brand identity.** Premium can extend brand for artistic effect, but should justify in spec.
- **Asking impossible from MJ.** "Generate exact stock price chart" — MJ can't do precise data. Use MJ for atmosphere, Figma for data.
- **Forgetting Figma details.** A spec without text positioning is half-done.
- **No reference examples.** Owner needs visual benchmark. Always cite 2-3 reference styles.
- **No estimated time.** Owner needs to budget. Always estimate.
- **No quality checklist.** Without checklist, owner doesn't know when "done".

## Storage in VisualArtifact

Premium spec stored as `content` in VisualArtifact:
- `format: "spec_json"`
- `tier: 5`
- `type: "premium_spec"`
- `generationMethod: "claude_spec_generation"`

When owner executes spec, the final image is uploaded separately as a new VisualArtifact:
- `format: "png"` or `format: "svg"`
- `tier: 5`
- `type: "premium_artifact"`
- `generationMethod: "midjourney_figma"`
- `manualWorkMinutes: <actual time>`

The two are linked via the `sourceId` (both point to same Analysis/Briefing/Artifact).

## Integration

- Used by `lib/visual.js` `generatePremiumSpec`
- Triggered by `/visual-spec <source_id>` command
- Output is JSON, not SVG
- Owner executes manually using external tools
- Final image uploaded back to system as separate VisualArtifact
