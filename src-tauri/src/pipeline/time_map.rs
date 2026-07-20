//! Source timeline ↔ output timeline mapping from EDL keep ranges.

use crate::models::edl::Edl;
use crate::models::event::Span;

/// Map built from ordered non-overlapping keep ranges on the source timeline.
#[derive(Debug, Clone)]
pub struct TimeMap {
    /// (source_start, source_end, output_start)
    ranges: Vec<(f64, f64, f64)>,
    pub source_duration: f64,
    pub output_duration: f64,
}

impl TimeMap {
    pub fn from_edl(edl: &Edl) -> Self {
        Self::from_keep_ranges(&edl.keep_ranges(), edl.source_duration)
    }

    pub fn from_keep_ranges(keep: &[(f64, f64)], source_duration: f64) -> Self {
        let mut ranges = Vec::new();
        let mut out_cursor = 0.0_f64;
        for &(s, e) in keep {
            if e <= s {
                continue;
            }
            ranges.push((s, e, out_cursor));
            out_cursor += e - s;
        }
        Self {
            ranges,
            source_duration: source_duration.max(0.0),
            output_duration: out_cursor,
        }
    }

    pub fn identity(duration: f64) -> Self {
        let d = duration.max(0.0);
        Self {
            ranges: if d > 0.0 {
                vec![(0.0, d, 0.0)]
            } else {
                vec![]
            },
            source_duration: d,
            output_duration: d,
        }
    }

    /// Map a source time to output time. Returns None if the instant was cut.
    pub fn source_to_output(&self, t: f64) -> Option<f64> {
        for &(s, e, o0) in &self.ranges {
            if t >= s && t < e {
                return Some(o0 + (t - s));
            }
            // allow exact end of last range
            if (t - e).abs() < 1e-6 {
                return Some(o0 + (e - s));
            }
        }
        None
    }

    /// Map output time back to source.
    pub fn output_to_source(&self, t: f64) -> Option<f64> {
        for &(s, e, o0) in &self.ranges {
            let len = e - s;
            if t >= o0 && t < o0 + len {
                return Some(s + (t - o0));
            }
            if (t - (o0 + len)).abs() < 1e-6 {
                return Some(e);
            }
        }
        None
    }

    /// Map a source span into zero or more output spans (split by cuts).
    pub fn map_source_span(&self, span: Span) -> Vec<Span> {
        let mut out = Vec::new();
        let a = span.start;
        let b = span.end;
        if b <= a {
            return out;
        }
        for &(s, e, o0) in &self.ranges {
            let is = a.max(s);
            let ie = b.min(e);
            if ie > is {
                let os = o0 + (is - s);
                let oe = o0 + (ie - s);
                out.push(Span::new(os, oe));
            }
        }
        out
    }

    /// Primary output span covering the longest surviving part of the source span.
    pub fn primary_output_span(&self, span: Span) -> Option<Span> {
        self.map_source_span(span)
            .into_iter()
            .max_by(|x, y| {
                x.duration()
                    .partial_cmp(&y.duration())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn is_fully_removed(&self, span: Span) -> bool {
        self.map_source_span(span).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_cut_middle() -> TimeMap {
        // keep 0-10 and 20-30 → output 0-10 then 10-20
        TimeMap::from_keep_ranges(&[(0.0, 10.0), (20.0, 30.0)], 30.0)
    }

    #[test]
    fn identity_maps() {
        let m = TimeMap::identity(60.0);
        assert!((m.source_to_output(12.5).unwrap() - 12.5).abs() < 1e-9);
        assert!((m.output_to_source(12.5).unwrap() - 12.5).abs() < 1e-9);
    }

    #[test]
    fn cut_region_returns_none() {
        let m = map_cut_middle();
        assert!(m.source_to_output(15.0).is_none());
        assert!((m.source_to_output(5.0).unwrap() - 5.0).abs() < 1e-9);
        assert!((m.source_to_output(25.0).unwrap() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn span_split_across_cut() {
        let m = map_cut_middle();
        let parts = m.map_source_span(Span::new(8.0, 22.0));
        assert_eq!(parts.len(), 2);
        assert!((parts[0].start - 8.0).abs() < 1e-6);
        assert!((parts[0].end - 10.0).abs() < 1e-6);
        assert!((parts[1].start - 10.0).abs() < 1e-6);
        assert!((parts[1].end - 12.0).abs() < 1e-6);
    }

    #[test]
    fn fully_removed_span() {
        let m = map_cut_middle();
        assert!(m.is_fully_removed(Span::new(11.0, 19.0)));
    }

    #[test]
    fn empty_keep() {
        let m = TimeMap::from_keep_ranges(&[], 10.0);
        assert!(m.source_to_output(1.0).is_none());
        assert!(m.map_source_span(Span::new(0.0, 5.0)).is_empty());
    }

    #[test]
    fn no_cuts_full_keep() {
        let m = TimeMap::from_keep_ranges(&[(0.0, 10.0)], 10.0);
        let p = m.map_source_span(Span::new(2.0, 4.0));
        assert_eq!(p.len(), 1);
        assert!((p[0].duration() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_and_zero_span() {
        let m = map_cut_middle();
        assert!(m.map_source_span(Span::new(5.0, 5.0)).is_empty());
        assert!(m.map_source_span(Span::new(6.0, 4.0)).is_empty());
    }

    #[test]
    fn exact_keep_boundary() {
        let m = map_cut_middle();
        // start of second keep
        assert!((m.source_to_output(20.0).unwrap() - 10.0).abs() < 1e-6);
        // end of first keep
        assert!((m.source_to_output(10.0).unwrap() - 10.0).abs() < 1e-6);
    }

    #[test]
    fn partial_removal_primary_span() {
        let m = map_cut_middle();
        // 8-15: only 8-10 survives in first keep
        let p = m.primary_output_span(Span::new(8.0, 15.0)).unwrap();
        assert!((p.start - 8.0).abs() < 1e-6);
        assert!((p.end - 10.0).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_kept_points() {
        let m = map_cut_middle();
        for t in [0.0, 3.5, 9.9, 20.1, 29.0] {
            let o = m.source_to_output(t).unwrap();
            let back = m.output_to_source(o).unwrap();
            assert!((back - t).abs() < 1e-6, "t={t} o={o} back={back}");
        }
    }

    #[test]
    fn skips_inverted_keep_entries() {
        let m = TimeMap::from_keep_ranges(&[(5.0, 3.0), (0.0, 2.0)], 10.0);
        assert!((m.output_duration - 2.0).abs() < 1e-9);
        assert!(m.source_to_output(1.0).is_some());
        assert!(m.source_to_output(4.0).is_none());
    }
}
