// ── Tests for diagram builtins (extracted from former diagrams.rs) ──
//
// Наряд №169: tests moved verbatim — no logic changes. The `super::*`
// import resolves to whatever diagrams/mod.rs re-exports, which means
// all builtin_diagram_* and builtin_infographic_qa functions remain
// accessible by their original names.

#[cfg(test)]
mod tests {
    #![allow(clippy::module_inception)]
    use super::super::super::shared::extract_style;
    use super::super::super::*; // svg module re-exports
    use crate::interpreter::Value;
    use std::collections::HashMap;

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }
    fn f(n: f64) -> Value {
        Value::Float(n)
    }
    #[allow(dead_code)]
    fn svg_rect_basic() {
        let out = builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("#eb6c36"), s("none")])
            .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<rect"#));
                assert!(xml.contains(r#"x="10""#));
                assert!(xml.contains(r#"width="100""#));
                assert!(xml.contains(r#"height="50""#));
                assert!(xml.contains(r##"fill="#eb6c36""##));
                assert!(xml.contains(r#"stroke="none""#));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_rect_rejects_zero_dimensions() {
        let r = builtin_svg_rect(&[f(0.0), f(0.0), f(0.0), f(50.0), s("red"), s("none")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_text_escapes_script_tag() {
        let out = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("<script>alert(1)</script>"),
            f(14.0),
            s("#2d3142"),
            s("start"),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                // Critical security invariant: < and > MUST be escaped
                assert!(!xml.contains("<script>"));
                assert!(xml.contains("&lt;script&gt;"));
                assert!(xml.contains("&lt;/script&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_text_escapes_quotes_and_ampersand() {
        let out = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("test \"quoted\" & <tag>"),
            f(14.0),
            s("#000"),
            s("start"),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains("&amp;"));
                assert!(xml.contains("&lt;tag&gt;"));
                assert!(xml.contains("&quot;quoted&quot;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_text_rejects_invalid_anchor() {
        let r = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("hello"),
            f(14.0),
            s("#000"),
            s("center"),
        ]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_canvas_returns_valid_xml_skeleton() {
        let child =
            builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("red"), s("none")]).unwrap();
        let out = builtin_svg_canvas(&[
            f(200.0),
            f(100.0),
            s("0 0 200 100"),
            Value::List(vec![child]),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
                assert!(xml.contains(r#"width="200""#));
                assert!(xml.contains(r#"height="100""#));
                assert!(xml.contains(r#"viewBox="0 0 200 100""#));
                assert!(xml.contains("<rect"));
                assert!(xml.ends_with("</svg>"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_canvas_rejects_invalid_viewbox() {
        let r = builtin_svg_canvas(&[f(200.0), f(100.0), s("0 0 200"), Value::List(vec![])]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_path_rejects_angle_brackets() {
        let r = builtin_svg_path(&[s("M 10 10 <script>"), s("none"), s("black")]);
        assert!(r.is_err());
    }

    #[test]
    fn diagram_style_returns_struct_with_5_tokens() {
        let mut fields = HashMap::new();
        fields.insert("paper".to_string(), s("#f5f5f5"));
        fields.insert("ink".to_string(), s("#2d3142"));
        fields.insert("accent".to_string(), s("#eb6c36"));
        fields.insert("muted".to_string(), s("#4f5d75"));
        fields.insert("rule".to_string(), s("rgba(45,49,66,0.12)"));
        let style_arg = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields,
        };
        let out = builtin_diagram_style(&[style_arg]).unwrap();
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DiagramStyle");
                assert_eq!(fields.len(), 5);
                assert!(fields.contains_key("paper"));
                assert!(fields.contains_key("ink"));
                assert!(fields.contains_key("accent"));
                assert!(fields.contains_key("muted"));
                assert!(fields.contains_key("rule"));
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn diagram_style_rejects_missing_token() {
        let mut fields = HashMap::new();
        fields.insert("paper".to_string(), s("#f5f5f5"));
        fields.insert("ink".to_string(), s("#2d3142"));
        // missing accent, muted, rule
        let style_arg = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields,
        };
        let r = builtin_diagram_style(&[style_arg]);
        assert!(r.is_err());
    }

    #[test]
    fn chart_bar_basic_3_bars() {
        let mut fields1 = HashMap::new();
        fields1.insert("label".to_string(), s("Янв"));
        fields1.insert("value".to_string(), f(40.0));
        let item1 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields1,
        };
        let mut fields2 = HashMap::new();
        fields2.insert("label".to_string(), s("Фев"));
        fields2.insert("value".to_string(), f(65.0));
        let item2 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields2,
        };
        let mut fields3 = HashMap::new();
        fields3.insert("label".to_string(), s("Мар"));
        fields3.insert("value".to_string(), f(30.0));
        let item3 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields3,
        };
        let data = Value::List(vec![item1, item2, item3]);

        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#f5f5f5"));
        style_fields.insert("ink".to_string(), s("#2d3142"));
        style_fields.insert("accent".to_string(), s("#eb6c36"));
        style_fields.insert("muted".to_string(), s("#4f5d75"));
        style_fields.insert("rule".to_string(), s("rgba(45,49,66,0.12)"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };

        let out = builtin_chart_bar(&[data, style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with(r#"<svg "#));
                assert!(xml.ends_with("</svg>"));
                // 3 bars (each contains <rect)
                let rect_count = xml.matches("<rect").count();
                assert!(rect_count >= 4); // 3 bars + 1 background = 4
                                          // Labels present and not escaped (Cyrillic is fine in XML UTF-8)
                assert!(xml.contains("Янв"));
                assert!(xml.contains("Фев"));
                assert!(xml.contains("Мар"));
                // The tallest bar (65) should be accent-colored
                assert!(xml.contains("fill=\"#eb6c36\""));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_bar_rejects_empty_data() {
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_bar(&[Value::List(vec![]), style]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_sketchy_filter_default_params() {
        let out = builtin_svg_sketchy_filter(&[s("sketch1")]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<filter id="sketch1">"#));
                assert!(xml.contains("feTurbulence"));
                assert!(xml.contains("feDisplacementMap"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_sketchy_filter_rejects_bad_id() {
        let r = builtin_svg_sketchy_filter(&[s("id with spaces")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_icon_known_name() {
        let out =
            builtin_svg_icon(&[s("server"), f(10.0), f(10.0), f(24.0), s("currentColor")]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<svg "#));
                assert!(xml.contains(r#"x="10""#));
                assert!(xml.contains(r#"y="10""#));
                assert!(xml.contains(r#"width="24""#));
                assert!(xml.contains(r#"height="24""#));
                assert!(xml.contains(r#"stroke="currentColor""#));
                assert!(xml.contains("<path"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_icon_unknown_name_errors() {
        let r = builtin_svg_icon(&[s("nonexistent"), f(0.0), f(0.0), f(24.0), s("black")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_callout_default_intent() {
        let out = builtin_svg_callout(&[s("note"), f(10.0), f(10.0), f(100.0), f(50.0)]).unwrap();
        match out {
            Value::String(xml) => {
                // Dashed line (callout invariant)
                assert!(xml.contains(r#"stroke-dasharray="3,3""#));
                // Italic text
                assert!(xml.contains(r#"font-style="italic""#));
                // Anchor dot
                assert!(xml.contains("<circle"));
                // Text content
                assert!(xml.contains("note"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_callout_escapes_text() {
        let out =
            builtin_svg_callout(&[s("<b>bold</b>"), f(10.0), f(10.0), f(100.0), f(50.0)]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(!xml.contains("<b>bold</b>"));
                assert!(xml.contains("&lt;b&gt;bold&lt;/b&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    // ── Наряд №77: color_palette + chart_donut unit tests ──

    #[test]
    fn color_palette_returns_diagram_style_struct_with_5_tokens() {
        let out = builtin_color_palette(&[s("energy"), s("light")]).unwrap();
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DiagramStyle");
                assert_eq!(fields.len(), 5);
                for k in &["paper", "ink", "accent", "muted", "rule"] {
                    assert!(fields.contains_key(*k), "missing token {}", k);
                }
                // Each token must be a hex string of form #rrggbb
                for k in &["paper", "ink", "accent", "muted", "rule"] {
                    if let Some(Value::String(v)) = fields.get(*k) {
                        assert!(v.starts_with('#'), "{} should start with #", k);
                        assert_eq!(v.len(), 7, "{} should be #rrggbb (7 chars)", k);
                    }
                }
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn color_palette_rejects_unknown_intent() {
        let r = builtin_color_palette(&[s("unknown"), s("light")]);
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(err.contains("intent"), "err: {}", err);
    }

    #[test]
    fn color_palette_rejects_unknown_mode() {
        let r = builtin_color_palette(&[s("calm"), s("neon")]);
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(err.contains("mode"), "err: {}", err);
    }

    #[test]
    fn color_palette_light_vs_dark_produce_different_tokens() {
        let light = builtin_color_palette(&[s("authority"), s("light")]).unwrap();
        let dark = builtin_color_palette(&[s("authority"), s("dark")]).unwrap();
        if let (Value::Struct { fields: lf, .. }, Value::Struct { fields: df, .. }) = (light, dark)
        {
            // Light paper should be much lighter than dark paper.
            // Value doesn't impl PartialEq, so extract strings and compare those.
            let lp = match lf.get("paper").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("light paper not String"),
            };
            let dp = match df.get("paper").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("dark paper not String"),
            };
            assert_ne!(lp, dp, "light vs dark paper must differ");
            let li = match lf.get("ink").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("light ink not String"),
            };
            let di = match df.get("ink").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("dark ink not String"),
            };
            assert_ne!(li, di, "light vs dark ink must differ");
        }
    }

    #[test]
    fn color_palette_all_6_intents_all_2_modes_produce_valid_hex() {
        // Наряд №162: mono added to the intent set
        for intent in &["calm", "tension", "energy", "authority", "warmth", "mono"] {
            for mode in &["light", "dark"] {
                let out = builtin_color_palette(&[s(intent), s(mode)]).unwrap();
                if let Value::Struct { fields, .. } = out {
                    for k in &["paper", "ink", "accent", "muted", "rule"] {
                        if let Some(Value::String(v)) = fields.get(*k) {
                            assert!(
                                v.starts_with('#') && v.len() == 7,
                                "intent={} mode={} token={} got {:?}",
                                intent,
                                mode,
                                k,
                                v
                            );
                            // Hex digits only after #
                            let hex = &v[1..];
                            assert!(
                                hex.chars().all(|c| c.is_ascii_hexdigit()),
                                "non-hex char in {} for intent={} mode={}",
                                k,
                                intent,
                                mode
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn color_palette_mono_exact_hex_tokens() {
        // Наряд №162: hand-picked values must not drift (not HSL-derived).
        let light = builtin_color_palette(&[s("mono"), s("light")]).unwrap();
        if let Value::Struct { fields, .. } = light {
            let get = |k: &str| match fields.get(k) {
                Some(Value::String(v)) => v.clone(),
                _ => panic!("missing {}", k),
            };
            assert_eq!(get("paper"), "#F0EFEB");
            assert_eq!(get("ink"), "#1C1C1A");
            assert_eq!(get("muted"), "#8F8E88");
            assert_eq!(get("rule"), "#DEDDD6");
            // accent = ink (no chromatic accent in mono aesthetic)
            assert_eq!(get("accent"), "#1C1C1A");
        } else {
            panic!("expected Struct");
        }

        let dark = builtin_color_palette(&[s("mono"), s("dark")]).unwrap();
        if let Value::Struct { fields, .. } = dark {
            let get = |k: &str| match fields.get(k) {
                Some(Value::String(v)) => v.clone(),
                _ => panic!("missing {}", k),
            };
            assert_eq!(get("paper"), "#1C1C1A");
            assert_eq!(get("ink"), "#F0EFEB");
            assert_eq!(get("muted"), "#8F8E88");
            assert_eq!(get("rule"), "#2E2D29");
            assert_eq!(get("accent"), "#F0EFEB");
        } else {
            panic!("expected Struct");
        }
    }

    #[test]
    fn color_palette_mono_rejects_bad_mode() {
        let r = builtin_color_palette(&[s("mono"), s("neon")]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("mode"));
    }

    #[test]
    fn color_palette_result_passes_extract_style() {
        // Critical: color_palette output must be consumable by extract_style
        // (the helper used by chart_bar / chart_donut).
        let out = builtin_color_palette(&[s("warmth"), s("light")]).unwrap();
        let extracted = extract_style(&out);
        assert!(extracted.is_ok(), "extract_style failed: {:?}", extracted);
        let style = extracted.unwrap();
        assert_eq!(style.len(), 5);
        for k in &["paper", "ink", "accent", "muted", "rule"] {
            assert!(style.contains_key(*k));
        }
    }

    #[test]
    fn color_palette_result_works_with_chart_bar() {
        // End-to-end: color_palette → chart_bar (no manual diagram_style needed)
        let palette = builtin_color_palette(&[s("energy"), s("dark")]).unwrap();
        let mut item_fields = HashMap::new();
        item_fields.insert("label".to_string(), s("Q1"));
        item_fields.insert("value".to_string(), f(40.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: item_fields,
        };
        let data = Value::List(vec![item]);
        let out = builtin_chart_bar(&[data, palette]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_basic_3_slices() {
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("Alpha"));
        f1.insert("value".to_string(), f(40.0));
        let item1 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut f2 = HashMap::new();
        f2.insert("label".to_string(), s("Beta"));
        f2.insert("value".to_string(), f(35.0));
        let item2 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f2,
        };
        let mut f3 = HashMap::new();
        f3.insert("label".to_string(), s("Gamma"));
        f3.insert("value".to_string(), f(25.0));
        let item3 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f3,
        };
        let data = Value::List(vec![item1, item2, item3]);

        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#eb6c36"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };

        let out = builtin_chart_donut(&[data, style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
                // 3 slices = 3 <path> elements (each donut slice is one path)
                let path_count = xml.matches("<path").count();
                assert_eq!(path_count, 3, "expected 3 slice paths");
                // Background rect
                assert!(xml.contains("<rect"));
                // Labels present (escaped if needed — Alpha/Beta/Gamma are safe)
                assert!(xml.contains("Alpha"));
                assert!(xml.contains("Beta"));
                assert!(xml.contains("Gamma"));
                // Center total: 40+35+25=100
                assert!(xml.contains(">100<"));
                // Legend swatches: 3 (one per slice)
                let rect_count = xml.matches("<rect").count();
                assert!(
                    rect_count >= 4,
                    "expected 4+ rects (1 bg + 3 legend swatches)"
                );
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_rejects_empty_data() {
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_donut(&[Value::List(vec![]), style]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("empty"));
    }

    #[test]
    fn chart_donut_rejects_negative_value() {
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("A"));
        f1.insert("value".to_string(), f(-10.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_donut(&[Value::List(vec![item]), style]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("non-negative"));
    }

    #[test]
    fn chart_donut_escapes_label_with_script_tag() {
        // Critical security invariant: <script> in label must NOT leak raw
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("<script>alert(1)</script>"));
        f1.insert("value".to_string(), f(40.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item]), style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(
                    !xml.contains("<script>"),
                    "RAW <script> leaked into chart_donut output: {}",
                    xml
                );
                assert!(xml.contains("&lt;script&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_single_slice_uses_accent() {
        // One slice = whole pie = accent color
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("Only"));
        f1.insert("value".to_string(), f(100.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#ff8800"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item]), style]).unwrap();
        match out {
            Value::String(xml) => {
                // The single slice should be filled with accent color
                assert!(
                    xml.contains(r##"fill="#ff8800""##),
                    "single slice should be accent-colored, xml: {}",
                    xml
                );
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_works_with_color_palette_output() {
        // End-to-end: color_palette → chart_donut
        let palette = builtin_color_palette(&[s("calm"), s("light")]).unwrap();
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("A"));
        f1.insert("value".to_string(), f(60.0));
        let item1 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut f2 = HashMap::new();
        f2.insert("label".to_string(), s("B"));
        f2.insert("value".to_string(), f(40.0));
        let item2 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f2,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item1, item2]), palette]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
                assert_eq!(xml.matches("<path").count(), 2, "expected 2 slice paths");
            }
            _ => panic!("expected String"),
        }
    }
}
