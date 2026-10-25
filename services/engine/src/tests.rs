use crate::{model::*, ranking::*};
fn slot() -> TimeSlot {
    serde_json::from_value(serde_json::json!({"id":"lunch","label":"Обед","start":"2026-11-01T13:00:00+03:00","end":"2026-11-01T15:00:00+03:00","timezone":"Europe/Moscow","requires_event":false,"interests":["джаз"],"criteria":{"center":{"lat":55.75,"lon":37.61},"radius_m":1000,"cuisines":["японская"],"max_price_rub":1500}})).unwrap()
}
fn place() -> PlaceCandidate {
    let s = slot();
    PlaceCandidate {
        id: "1".into(),
        name: "Test".into(),
        address: "Test".into(),
        coordinates: s.criteria.center.clone(),
        cuisines: vec!["Японская".into()],
        average_price_rub: Some(1000),
        opening_windows: vec![TimeWindow {
            start: s.start,
            end: s.end,
        }],
        website: Some("https://example.com".into()),
        menu_url: Some("https://example.com/menu".into()),
        menu_with_prices: true,
        relevance: 1.0,
        load: vec![LoadSample {
            start: s.start,
            end: s.end,
            value: 0.25,
        }],
        events: vec![],
        demo: false,
        evidence: vec![],
        discovery_provider: None,
        geocoding_source: None,
    }
}
#[test]
fn scenario_b_formula() {
    let p = place();
    let score = calculate_score(&p, &slot());
    assert_eq!(score.score, 92.5);
    assert_eq!(score.breakdown.event, 0.0);
    assert!(!score.breakdown.load_estimated);
}
#[test]
fn scenario_a_formula_and_absent_event() {
    let p = place();
    let mut s = slot();
    s.requires_event = true;
    let score = calculate_score(&p, &s);
    assert_eq!(score.score, 55.0);
    assert_eq!(score.breakdown.scenario, "A");
}
#[test]
fn matching_and_background_events() {
    let mut p = place();
    let mut s = slot();
    s.requires_event = true;
    p.events = vec![Event {
        title: "Jazz".into(),
        start: s.start,
        end: s.end,
        interests: vec!["джаз".into()],
        url: None,
    }];
    assert_eq!(calculate_score(&p, &s).score, 95.0);
    p.events[0].interests = vec!["стендап".into()];
    assert_eq!(calculate_score(&p, &s).breakdown.event, 20.0);
    p.events[0].start = s.end;
    p.events[0].end = s.end + chrono::Duration::hours(1);
    assert_eq!(calculate_score(&p, &s).breakdown.event, 0.0);
}
#[test]
fn high_score_never_overrides_filters() {
    let mut p = place();
    let s = slot();
    p.average_price_rub = Some(9999);
    assert!(apply_hard_filter(&[p.clone()], &s).is_empty());
    p.average_price_rub = None;
    assert_eq!(rejection_reason(&p, &s), Some("price_exceeded_or_unknown"));
    p.average_price_rub = Some(1000);
    p.cuisines.clear();
    assert!(rejection_reason(&p, &s).is_some());
}
#[test]
fn unknown_hours_and_full_interval() {
    let mut p = place();
    let s = slot();
    p.opening_windows[0].end = s.end - chrono::Duration::minutes(1);
    assert_eq!(rejection_reason(&p, &s), Some("closed_or_hours_unknown"));
    p.opening_windows.clear();
    assert!(rejection_reason(&p, &s).is_some());
}
#[test]
fn continuous_adjacent_windows() {
    let mut p = place();
    let s = slot();
    let mid = s.start + chrono::Duration::hours(1);
    p.opening_windows = vec![
        TimeWindow {
            start: s.start,
            end: mid,
        },
        TimeWindow {
            start: mid,
            end: s.end,
        },
    ];
    assert_eq!(rejection_reason(&p, &s), None);
}
#[test]
fn missing_and_partial_load() {
    let mut p = place();
    let s = slot();
    p.load.clear();
    let v = calculate_score(&p, &s);
    assert_eq!(v.breakdown.load_factor, 0.5);
    assert!(v.breakdown.load_estimated);
    p.load.push(LoadSample {
        start: s.start,
        end: s.start + chrono::Duration::hours(1),
        value: 0.0,
    });
    let v = calculate_score(&p, &s);
    assert_eq!(v.breakdown.load_factor, 0.25);
    assert!(v.breakdown.load_estimated);
}
#[test]
fn invalid_numeric_input_is_bounded() {
    let mut p = place();
    p.relevance = f64::NAN;
    p.load[0].value = f64::INFINITY;
    let v = calculate_score(&p, &slot());
    assert!(v.score.is_finite());
    assert!((0.0..=100.0).contains(&v.score));
    p.coordinates.lat = f64::NAN;
    assert_eq!(rejection_reason(&p, &slot()), Some("outside_radius"));
}
#[test]
fn deterministic_sort_and_no_duplicate_cards() {
    let p = place();
    let mut q = place();
    q.id = "0".into();
    let r = rank(&RankRequest {
        slots: vec![slot()],
        candidates: vec![p.clone(), q, p],
    })
    .unwrap();
    assert_eq!(r[0].cards.len(), 2);
    assert_eq!(r[0].cards[0].place.id, "0");
}
#[test]
fn timezone_and_duplicate_slot_validation() {
    let mut s = slot();
    s.timezone = "America/New_York".into();
    assert!(validate_slot(&s).is_err());
    let s = slot();
    assert!(
        rank(&RankRequest {
            slots: vec![s.clone(), s],
            candidates: vec![]
        })
        .is_err()
    );
}
#[test]
fn empty_results_have_explanation_and_buttons() {
    let r = rank(&RankRequest {
        slots: vec![slot()],
        candidates: vec![],
    })
    .unwrap();
    assert!(!r[0].text.is_empty());
    assert!(!r[0].dynamic_buttons.is_empty());
}
#[test]
fn unsafe_urls_cannot_earn_website_points() {
    let mut p = place();
    p.website = Some("javascript:alert(1)".into());
    p.menu_url = None;
    assert_eq!(calculate_score(&p, &slot()).breakdown.website_menu, 0.0);
}
#[test]
fn partial_profile_preserves_fields() {
    let mut p = crate::dialogue::Profile::default();
    p.merge(serde_json::json!({"location":"Москва","max_price_rub":1000}))
        .unwrap();
    p.merge(serde_json::json!({"cuisines":["японская"]}))
        .unwrap();
    assert_eq!(p.location.as_deref(), Some("Москва"));
    assert_eq!(p.max_price_rub, Some(1000));
}
#[test]
fn yandex_hours_seconds_and_weekends() {
    let s = slot();
    let meta = serde_json::json!({"Hours":{"Availabilities":[{"Weekend":true,"Intervals":[{"from":"10:00:00","to":"21:30:00"}]}]}});
    let w = crate::providers::opening_windows(&meta, &s);
    assert!(!w.is_empty());
    let mut p = place();
    p.opening_windows = w;
    assert_eq!(rejection_reason(&p, &s), None);
}
#[test]
fn overnight_hours_cover_next_day() {
    let mut s = slot();
    s.start = "2026-11-02T00:30:00+03:00".parse().unwrap();
    s.end = "2026-11-02T01:30:00+03:00".parse().unwrap();
    let meta = serde_json::json!({"Hours":{"Availabilities":[{"Sunday":true,"Intervals":[{"from":"20:00:00","to":"02:00:00"}]}]}});
    let mut p = place();
    p.opening_windows = crate::providers::opening_windows(&meta, &s);
    assert_eq!(rejection_reason(&p, &s), None);
}
#[test]
fn menu_without_prices_differs_between_scenarios() {
    let mut p = place();
    p.menu_with_prices = false;
    let mut s = slot();
    assert_eq!(calculate_score(&p, &s).breakdown.website_menu, 30.0);
    s.requires_event = true;
    assert_eq!(calculate_score(&p, &s).breakdown.website_menu, 10.0);
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL, run explicitly against an isolated PostgreSQL database"]
async fn database_owner_isolation_and_optimistic_lock() {
    let pool = crate::db::init(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
    let owner = uuid::Uuid::new_v4().to_string();
    let c = crate::db::create(pool.clone(), owner.clone())
        .await
        .unwrap();
    assert_eq!(
        crate::db::get(pool.clone(), "different-account".into(), c.id)
            .await
            .unwrap_err()
            .0,
        axum::http::StatusCode::NOT_FOUND
    );
    crate::db::save(
        pool.clone(),
        owner.clone(),
        c.id,
        0,
        serde_json::json!({"messages":[{"text":"persisted"}]}),
        "Saved".into(),
    )
    .await
    .unwrap();
    assert_eq!(
        crate::db::save(
            pool.clone(),
            owner.clone(),
            c.id,
            0,
            serde_json::json!({}),
            "overwrite".into()
        )
        .await
        .unwrap_err()
        .0,
        axum::http::StatusCode::CONFLICT
    );
    let saved = crate::db::get(pool.clone(), owner.clone(), c.id)
        .await
        .unwrap();
    assert_eq!(saved.document["messages"][0]["text"], "persisted");
    assert_eq!(
        crate::db::list(pool, "different-account".into(), 0)
            .await
            .unwrap()
            .iter()
            .filter(|x| x.id == c.id)
            .count(),
        0
    );
}
#[tokio::test]
async fn http_auth_and_rank_contract() {
    use axum::{
        Json, Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::get,
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let auth = Router::new().route(
        "/api/auth/users/me",
        get(|h: axum::http::HeaderMap| async move {
            if h.get("authorization").and_then(|x| x.to_str().ok()) == Some("Bearer accepted") {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({"user_id":7,"status":"active"})),
                )
            } else {
                (StatusCode::UNAUTHORIZED, Json(serde_json::json!({})))
            }
        }),
    );
    let task = tokio::spawn(async move { axum::serve(listener, auth).await.unwrap() });
    let manager =
        diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new("postgres://unused/unused");
    let app = Arc::new(crate::App {
        db: diesel::r2d2::Pool::builder()
            .max_size(1)
            .min_idle(Some(0))
            .build_unchecked(manager),
        client: reqwest::Client::new(),
        auth_url: format!("http://{address}"),
    });
    let router = crate::router(app);
    let body = serde_json::to_vec(&RankRequest {
        slots: vec![slot()],
        candidates: vec![place()],
    })
    .unwrap();
    for (token, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("Bearer forged"), StatusCode::UNAUTHORIZED),
        (Some("Bearer accepted"), StatusCode::OK),
    ] {
        let mut request = Request::builder()
            .uri("/api/v1/rank")
            .method("POST")
            .header("content-type", "application/json");
        if let Some(token) = token {
            request = request.header("authorization", token);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::from(body.clone())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let v: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap())
                    .unwrap();
            assert_eq!(v["slots"][0]["cards"][0]["score"], 92.5);
        }
    }
    task.abort();
}

#[test]
fn gemini_transport_uses_grounding_and_stateless_interactions() {
    use crate::gemini::{Mode, request_body};
    let (url, body) = request_body(
        "https://example.test/v1beta",
        "gemini-3.8-flash",
        false,
        Mode::Search,
        "Research",
        "Find cafes",
        None,
    );
    assert!(url.ends_with("/interactions"));
    assert_eq!(body["store"], false);
    assert_eq!(body["tools"][0]["type"], "google_search");
    assert!(body.get("response_format").is_none());
    let (_, body) = request_body(
        "https://example.test",
        "model",
        false,
        Mode::Function,
        "Extract",
        "Input",
        Some(crate::providers::query_places_schema()),
    );
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["parameters"]["type"], "object");
    assert_eq!(body["generation_config"]["tool_choice"], "any");
    let (_, body) = request_body(
        "https://example.test",
        "model",
        true,
        Mode::Search,
        "Research",
        "Input",
        None,
    );
    assert!(body["tools"][0].get("google_search").is_some());
}
#[test]
fn gemini_function_call_normalizes_both_api_versions() {
    use serde_json::json;
    let a = json!({"steps":[{"type":"function_call","name":"query_places","arguments":{"location":"Москва"}}]});
    let b = json!({"candidates":[{"content":{"parts":[{"functionCall":{"name":"query_places","args":{"location":"Москва"}}}]}}]});
    assert_eq!(
        crate::gemini::function_args(&a),
        crate::gemini::function_args(&b)
    );
}
#[test]
fn google_citations_support_unicode_and_do_not_accept_uncited_text() {
    use serde_json::json;
    let text = "Кафе Тест. Москва, Тестовая улица, 1.";
    let response = json!({"steps":[{"type":"model_output","content":[{"type":"text","text":text,"annotations":[{"type":"url_citation","url":"https://example.test/contact","title":"Venue","start_index":0,"end_index":text.chars().count()}]}]}]});
    let research = crate::web_search::parse_research(&response);
    assert_eq!(research.passages[0].text, text);
    let valid = crate::web_search::Citation {
        field: "identity".into(),
        source_index: 0,
        quote: text.into(),
    };
    assert_eq!(
        crate::web_search::accepted_evidence(&[valid], &research, "now").len(),
        1
    );
    let unsupported = crate::web_search::Citation {
        field: "average_price_rub".into(),
        source_index: 0,
        quote: "Средний чек 1000 рублей".into(),
    };
    assert!(crate::web_search::accepted_evidence(&[unsupported], &research, "now").is_empty());
}
#[test]
fn legacy_google_citations_reject_invalid_indices_and_schemes() {
    use serde_json::json;
    let response = json!({"candidates":[{"content":{"parts":[{"text":"A supported sentence"}]},"groundingMetadata":{"groundingChunks":[{"web":{"uri":"javascript:alert(1)","title":"bad"}},{"web":{"uri":"https://example.test","title":"ok"}}],"groundingSupports":[{"segment":{"text":"A supported sentence"},"groundingChunkIndices":[1,999]}]}}]});
    let r = crate::web_search::parse_research(&response);
    assert_eq!(r.sources.len(), 1);
    assert_eq!(r.passages[0].sources, vec![0]);
}
#[test]
fn unknown_price_and_hours_cannot_be_filled_to_fit_the_request() {
    use crate::web_search::{Citation, FoundPlace, Passage, Research, sanitize};
    let quote = "Кафе Тест, Москва, Тестовая улица, 1. Японская кухня.";
    let r = Research {
        text: quote.into(),
        sources: vec![WebSource {
            title: "Source".into(),
            url: "https://example.test".into(),
        }],
        passages: vec![Passage {
            text: quote.into(),
            sources: vec![0],
        }],
        queries: vec![],
        suggestions: vec![],
    };
    let found = FoundPlace {
        slot_id: "lunch".into(),
        name: "Кафе Тест".into(),
        address: "Москва, Тестовая улица, 1".into(),
        cuisines: vec!["японская".into()],
        average_price_rub: Some(1000),
        opening_windows: place().opening_windows,
        website: None,
        menu_url: None,
        menu_with_prices: true,
        events: vec![],
        evidence: vec![Citation {
            field: "identity".into(),
            source_index: 0,
            quote: quote.into(),
        }],
    };
    let p = sanitize(&found, &r, "now").unwrap();
    assert!(p.average_price_rub.is_none());
    assert!(p.opening_windows.is_empty());
    assert!(p.cuisines.is_empty());
    assert!(p.load.is_empty());
    assert!(!p.menu_with_prices);
    assert!(!p.demo);
    let mut bad = found;
    bad.name = "Выдуманное кафе".into();
    assert!(sanitize(&bad, &r, "now").is_none());
}
#[test]
fn gemini_errors_are_actionable_without_echoing_provider_payloads() {
    use serde_json::json;
    let r =
        crate::gemini::provider_error(429, &json!({"error":{"message":"secret-provider-payload"}}));
    assert_eq!(r.0, axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert!(!r.1.contains("secret"));
    let r = crate::gemini::provider_error(
        400,
        &json!({"error":{"message":"User location is not supported for the API use."}}),
    );
    assert!(r.1.contains("регион"));
}
#[test]
fn photon_coordinates_have_correct_axis_order_and_validate_bounds() {
    use serde_json::json;
    let c = crate::geocoding::parse_coordinates(
        &json!({"features":[{"geometry":{"coordinates":[30.33,59.93]}}]}),
    )
    .unwrap();
    assert_eq!(c.lat, 59.93);
    assert_eq!(c.lon, 30.33);
    assert!(
        crate::geocoding::parse_coordinates(
            &json!({"features":[{"geometry":{"coordinates":[300,59]}}]})
        )
        .is_err()
    );
    assert!(crate::geocoding::parse_coordinates(&json!({"features":[]})).is_err());
}

#[tokio::test]
#[ignore = "run with GEMINI_API_KEY=test GEMINI_API_BASE_URL=http://127.0.0.1:18888 GEOCODER_URL=http://127.0.0.1:18888/geocode PLACES_PROVIDER=gemini GEMINI_API_MODE=interactions"]
async fn grounded_search_pipeline_with_mock_providers() {
    use axum::{
        Json, Router,
        routing::{get, post},
    };
    use serde_json::{Value, json};
    assert_eq!(std::env::var("GEMINI_API_KEY").unwrap(), "test");
    assert_eq!(
        std::env::var("GEMINI_API_BASE_URL").unwrap(),
        "http://127.0.0.1:18888"
    );
    let quote = "Кафе Тест, Москва, Тестовая улица, 1. Японская кухня. Средний чек 1000 рублей. Ежедневно 09:00–23:00. Официальное меню с ценами: https://example.test/menu";
    let quote = quote.to_string();
    let profile = json!({"location":"Москва","cuisines":["японская"],"max_price_rub":1500,"requires_event":false,"slots":[{"id":"lunch","label":"Обед","start":"2026-11-01T13:00:00+03:00","end":"2026-11-01T15:00:00+03:00","timezone":"Europe/Moscow"}]});
    let evidence = [
        "identity",
        "cuisines",
        "average_price_rub",
        "opening_windows",
        "menu_url",
        "menu_with_prices",
    ]
    .iter()
    .map(|field| json!({"field":field,"source_index":0,"quote":quote}))
    .collect::<Vec<_>>();
    let result=json!({"places":[{"slot_id":"lunch","name":"Кафе Тест","address":"Москва, Тестовая улица, 1","cuisines":["японская"],"average_price_rub":1000,"opening_windows":[{"start":"2026-11-01T09:00:00+03:00","end":"2026-11-01T23:00:00+03:00"}],"website":null,"menu_url":"https://example.test/menu","menu_with_prices":true,"events":[],"evidence":evidence}]}).to_string();
    let quote_clone = quote.clone();
    let mock=Router::new().route("/interactions",post(move|Json(request):Json<Value>|{let profile=profile.clone();let quote=quote_clone.clone();let result=result.clone();async move{
   assert_eq!(request["store"],false);
   Json(if request["tools"][0]["type"]=="function"{json!({"steps":[{"type":"function_call","name":"query_places","arguments":profile}]})}
   else if request["tools"][0]["type"]=="google_search"{json!({"steps":[{"type":"google_search_call","arguments":{"queries":["Москва рамен меню"]}},{"type":"google_search_result","result":[{"search_suggestions":"<a href='https://google.com'>Search</a>"}]},{"type":"model_output","content":[{"type":"text","text":quote,"annotations":[{"type":"url_citation","url":"https://example.test/menu","title":"Test menu","start_index":0,"end_index":quote.chars().count()}]}]}]})}
   else{assert_eq!(request["response_format"]["mime_type"],"application/json");json!({"steps":[{"type":"model_output","content":[{"type":"text","text":result}]}]})})
 }})).route("/geocode",get(||async{Json(json!({"features":[{"properties":{"type":"house"},"geometry":{"coordinates":[37.61,55.75]}}]}))}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18888")
        .await
        .unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
    let client = reqwest::Client::new();
    let mut profile = crate::dialogue::Profile::default();
    let parsed = crate::providers::extract(&client, &profile, &[], "Хочу рамен на обед")
        .await
        .unwrap();
    profile.merge(parsed).unwrap();
    let response = crate::dialogue::answer(&client, &profile).await.unwrap();
    let card = &response["slots"][0]["cards"][0];
    assert_eq!(card["place"]["name"], "Кафе Тест");
    assert_eq!(card["score"], 85.0);
    assert_eq!(card["breakdown"]["load_estimated"], true);
    assert_eq!(card["place"]["demo"], false);
    assert!(!card["place"]["evidence"].as_array().unwrap().is_empty());
    assert_eq!(
        response["slots"][0]["grounding"]["sources"][0]["title"],
        "Test menu"
    );
    server.abort();
}
#[test]
fn slot_specific_preferences_skip_already_answered_questions() {
    let mut p = crate::dialogue::Profile::default();
    p.merge(serde_json::json!({"location":"Москва","slots":[{"id":"lunch","cuisines":["японская"],"max_price_rub":1500,"requires_event":false}]})).unwrap();
    assert!(p.question().is_none());
}
