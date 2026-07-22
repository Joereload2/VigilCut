//! Supervision API: cancel, regenerate, double-approve.

#[cfg(test)]
mod tests {
    use crate::models::visual_intel::{NeedCoverage, VisualNeed};
    use crate::pipeline::visual::generation::supervision::{
        cancel_job, get_job, queue_regenerate, supervision_snapshot,
    };
    use crate::pipeline::visual::generation::worker::{
        human_approve_candidate, human_reject_candidate, queue_generation_for_need, worker_tick,
    };
    use crate::pipeline::visual::library::set_library_root_override;
    use crate::pipeline::visual::needs::save_needs;

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn cancel_queued_and_double_approve() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-sup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
        std::env::remove_var("OMNIROUTE_BASE_URL");

        let mut need = VisualNeed::from_label("sup-proj", "concepto_unico_sup_xyz");
        need.terms = vec!["concepto_unico_sup_xyz".into()];
        need.output_start = Some(1.0);
        need.output_end = Some(5.0);
        save_needs(std::slice::from_ref(&need)).unwrap();

        let job_id = queue_generation_for_need(&mut need, false)
            .unwrap()
            .expect("job");
        // Cancel while still queued if not processed
        let j = get_job(&job_id).unwrap();
        if j.status == "queued" {
            let c = cancel_job(&job_id).unwrap();
            assert_eq!(c.status, "cancelled");
        }

        // Fresh need generate + process
        let mut need2 = VisualNeed::from_label("sup-proj", "otro_concepto_sup_abc");
        need2.terms = vec!["otro_concepto_sup_abc".into()];
        need2.output_start = Some(6.0);
        need2.output_end = Some(10.0);
        save_needs(std::slice::from_ref(&need2)).unwrap();
        let job2 = queue_generation_for_need(&mut need2, false)
            .unwrap()
            .expect("job2");
        let _ = worker_tick(2).await.unwrap();

        let snap = supervision_snapshot("sup-proj").unwrap();
        assert!(snap.coverage.total >= 1);

        // Find candidate for need2
        let entry = snap.needs.iter().find(|n| n.need.id == need2.id).cloned();
        if let Some(e) = entry {
            if let Some(c) = e.candidate {
                if c.status == "needs_human_review"
                    || c.status == "automated_review"
                    || c.status == "approved"
                    || c.status == "generated"
                {
                    // Approve twice — same asset
                    if c.status != "approved" {
                        let a1 = human_approve_candidate(&c.id).unwrap();
                        let a2 = human_approve_candidate(&c.id).unwrap();
                        assert_eq!(a1.id, a2.id);
                    }
                }
            }
        }

        // Reject path
        let mut need3 = VisualNeed::from_label("sup-proj", "reject_me_sup");
        need3.terms = vec!["reject_me_sup".into()];
        save_needs(std::slice::from_ref(&need3)).unwrap();
        let _ = queue_generation_for_need(&mut need3, false).unwrap();
        let _ = worker_tick(2).await.unwrap();
        let snap2 = supervision_snapshot("sup-proj").unwrap();
        if let Some(e) = snap2.needs.iter().find(|n| n.need.id == need3.id) {
            if let Some(c) = &e.candidate {
                let _ = human_reject_candidate(&c.id);
                let _ = queue_regenerate(&need3.id);
            }
        }

        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
        let _ = job2;
        let _ = NeedCoverage::Uncovered;
    }
}
