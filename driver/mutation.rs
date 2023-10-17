use json_patch::{
    AddOperation,
    Patch,
    PatchOperation,
};
use k8s_openapi::api::core::v1 as corev1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as metav1;
use kube::core::admission::{
    AdmissionRequest,
    AdmissionResponse,
    AdmissionReview,
};
use kube::ResourceExt;
use rocket::serde::json::Json;
use serde_json::{
    json,
    Value,
};
use simkube::jsonutils;
use simkube::prelude::*;
use tracing::*;

use super::*;

#[rocket::post("/", data = "<body>")]
pub async fn handler(
    ctx: &rocket::State<DriverContext>,
    body: Json<AdmissionReview<corev1::Pod>>,
) -> Json<AdmissionReview<corev1::Pod>> {
    let req: AdmissionRequest<_> = match body.into_inner().try_into() {
        Ok(r) => r,
        Err(err) => {
            error!("could not parse request: {err:?}");
            let resp = AdmissionResponse::invalid(err);
            return Json(into_pod_review(resp));
        },
    };

    let mut resp = AdmissionResponse::from(&req);
    if let Some(pod) = &req.object {
        info!("received mutation request for pod: {}", pod.namespaced_name());
        resp = match mutate_pod(ctx, resp, pod).await {
            Ok(r) => {
                info!("mutation successfully constructed");
                r
            },
            Err(err) => {
                error!("could not perform mutation, blocking pod object: {err:?}");
                AdmissionResponse::from(&req).deny(err)
            },
        };
    }

    Json(into_pod_review(resp))
}

// TODO when we get the pod object, the final name hasn't been filled in yet; make sure this
// doesn't cause any problems
pub(super) async fn mutate_pod(
    ctx: &DriverContext,
    resp: AdmissionResponse,
    pod: &corev1::Pod,
) -> anyhow::Result<AdmissionResponse> {
    // enclose in a block so we release the mutex when we're done
    let owners = {
        let mut owners_cache = ctx.owners_cache.lock().await;
        owners_cache.compute_owner_chain(pod).await?
    };

    if !owners.iter().any(|o| o.name == ctx.sim_root) {
        return Ok(resp);
    }

    let mut patches = vec![];
    add_simulation_labels(ctx, pod, &mut patches)?;
    add_lifecycle_annotation(ctx, pod, &owners, &mut patches)?;
    add_node_selector_tolerations(pod, &mut patches)?;

    Ok(resp.with_patch(Patch(patches))?)
}

fn add_simulation_labels(ctx: &DriverContext, pod: &corev1::Pod, patches: &mut Vec<PatchOperation>) -> EmptyResult {
    if pod.metadata.labels.is_none() {
        patches.push(PatchOperation::Add(AddOperation { path: "/metadata/labels".into(), value: json!({}) }));
    }
    patches.push(PatchOperation::Add(AddOperation {
        path: format!("/metadata/labels/{}", jsonutils::escape(SIMULATION_LABEL_KEY)),
        value: Value::String(ctx.name.clone()),
    }));

    Ok(())
}

fn add_lifecycle_annotation(
    ctx: &DriverContext,
    pod: &corev1::Pod,
    owners: &Vec<metav1::OwnerReference>,
    patches: &mut Vec<PatchOperation>,
) -> EmptyResult {
    if let Some(orig_ns) = pod.annotations().get(ORIG_NAMESPACE_ANNOTATION_KEY) {
        for owner in owners {
            let owner_ns_name = format!("{}/{}", orig_ns, owner.name);
            let lifecycle = ctx.store.lookup_pod_lifecycle(pod, &owner_ns_name, 0)?;
            if let Some(patch) = lifecycle.to_annotation_patch() {
                if pod.metadata.annotations.is_none() {
                    patches.push(PatchOperation::Add(AddOperation {
                        path: "/metadata/annotations".into(),
                        value: json!({}),
                    }));
                }
                patches.push(patch);
                break;
            }
        }
    }

    warn!("no pod lifecycle data found for {}", pod.namespaced_name());
    Ok(())
}

fn add_node_selector_tolerations(pod: &corev1::Pod, patches: &mut Vec<PatchOperation>) -> EmptyResult {
    if pod.spec()?.tolerations.is_none() {
        patches.push(PatchOperation::Add(AddOperation { path: "/spec/tolerations".into(), value: json!([]) }));
    }
    patches.push(PatchOperation::Add(AddOperation {
        path: "/spec/nodeSelector".into(),
        value: json!({"type": "virtual"}),
    }));
    patches.push(PatchOperation::Add(AddOperation {
        path: "/spec/tolerations/-".into(),
        value: json!({"key": VIRTUAL_NODE_TOLERATION_KEY, "value": "true"}),
    }));

    Ok(())
}

// Have to duplicate this fn because AdmissionResponse::into_review uses the dynamic API
fn into_pod_review(resp: AdmissionResponse) -> AdmissionReview<corev1::Pod> {
    AdmissionReview {
        types: resp.types.clone(),
        // All that matters is that we keep the request UUID, which is in the TypeMeta
        request: None,
        response: Some(resp),
    }
}