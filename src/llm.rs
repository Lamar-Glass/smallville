use serde_json::json;

const MOCK_KEY: &str = "sk-mock";

pub struct LlmClient {
    api_key: String,
    base_url: String,
    model: String,
    embed_model: String,
    pub mock: bool,
}

impl LlmClient {
    pub fn from_env(model: &str, embed_model: &str, base_url: &str) -> Result<Self, String> {
        let mock = std::env::var("SIM_MOCK").map(|v| v == "1").unwrap_or(false);
        let api_key = if mock {
            MOCK_KEY.to_string()
        } else {
            std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("RUSTY_LLM_API_KEY"))
                .map_err(|_| {
                    "No API key found. Set OPENAI_API_KEY (or RUSTY_LLM_API_KEY), \
                     or set SIM_MOCK=1 to run a scripted town with no API calls."
                        .to_string()
                })?
        };
        Ok(Self {
            api_key,
            base_url: base_url.to_string(),
            model: model.to_string(),
            embed_model: embed_model.to_string(),
            mock,
        })
    }

    pub fn chat(&self, system: &str, user: &str) -> Result<String, String> {
        if self.mock {
            return Ok(mock_chat(user));
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(json!({
                "model": self.model,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user }
                ],
                "temperature": 0.8,
            }));
        let body = match resp {
            Ok(r) => r
                .into_string()
                .map_err(|e| format!("failed to read LLM response: {e}"))?,
            Err(ureq::Error::Status(code, r)) => {
                return Err(format!(
                    "LLM API error ({code}): {}",
                    r.into_string().unwrap_or_default()
                ))
            }
            Err(e) => return Err(format!("LLM request failed: {e}")),
        };
        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("bad LLM json: {e}"))?;
        value["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| format!("unexpected LLM response: {body}"))
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        if self.mock {
            return Ok(mock_embed(text));
        }
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(json!({ "model": self.embed_model, "input": text }));
        let body = match resp {
            Ok(r) => r
                .into_string()
                .map_err(|e| format!("failed to read embedding response: {e}"))?,
            Err(ureq::Error::Status(code, r)) => {
                return Err(format!(
                    "embedding API error ({code}): {}",
                    r.into_string().unwrap_or_default()
                ))
            }
            Err(e) => return Err(format!("embedding request failed: {e}")),
        };
        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("bad embedding json: {e}"))?;
        value["data"][0]["embedding"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_f64())
                    .map(|x| x as f32)
                    .collect()
            })
            .ok_or_else(|| format!("unexpected embedding response: {body}"))
    }
}

fn mock_chat(user: &str) -> String {
    let lowered = user.to_lowercase();
    if lowered.contains("daily schedule") || lowered.contains("schedule") {
        let home = user
            .lines()
            .find(|l| l.to_lowercase().contains("your home is"))
            .and_then(|l| {
                l.split_once("Your home is ")
                    .or_else(|| l.split_once("your home is "))
                    .and_then(|(_, rest)| rest.split(" and you usually").next())
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_default();
        let work = user
            .lines()
            .find(|l| l.to_lowercase().contains("work around"))
            .and_then(|l| {
                l.split(" work around ")
                    .nth(1)
                    .map(|s| s.trim().trim_end_matches('.').to_string())
            })
            .unwrap_or_default();
        let h = if home.is_empty() { "The Lin Family House" } else { &home };
        let w = if work.is_empty() { "Willow Market" } else { &work };
        return [
            format!("08:00|{h}|having breakfast and reading the paper"),
            format!("09:00|{w}|getting settled and checking on things"),
            format!("10:00|{w}|working and helping neighbors"),
            "11:00|Hobbit Hole Café|taking a coffee break".to_string(),
            "12:00|Hobbit Hole Café|having lunch and catching up with friends".to_string(),
            format!("13:00|{w}|getting back to work"),
            format!("14:00|{w}|working through the afternoon"),
            "15:00|The Park|taking a walk and enjoying the weather".to_string(),
            format!("16:00|{w}|wrapping up the day's tasks"),
            "17:00|Willow Market|running errands and picking up groceries".to_string(),
            format!("18:00|{h}|preparing dinner"),
            format!("19:00|{h}|eating dinner"),
            format!("20:00|{h}|relaxing and reading"),
            format!("21:00|{h}|unwinding before bed"),
            format!("22:00|{h}|getting ready to sleep"),
        ]
        .join("\n");
    }
    if lowered.contains("reflect") {
        return [
            "I value my quiet routines but should say hello to neighbors more.",
            "The café is where the town really comes together.",
            "A little help at the market goes a long way.",
        ]
        .join("\n");
    }
    if lowered.contains("dialogue") {
        let names: Vec<String> = user
            .lines()
            .take(3)
            .filter_map(|l| {
                if l.starts_with("Dialogue between: ") {
                    Some(l.trim_start_matches("Dialogue between: ").to_string())
                } else {
                    None
                }
            })
            .next()
            .map(|n| n.split(", ").map(|s| s.to_string()).collect())
            .unwrap_or_default();
        if names.len() >= 2 {
            return format!(
                "{}: The flowers in the park are looking wonderful today.\n{}: They are! I helped water them this morning.",
                names[0], names[1]
            );
        }
        return "Person A: Nice day, isn't it?\nPerson B: It really is.".to_string();
    }
    "Smallville had a busy day of routines, errands, and friendly conversation at the café."
        .to_string()
}

fn mock_embed(text: &str) -> Vec<f32> {
    text.bytes()
        .enumerate()
        .map(|(i, b)| ((b as f32) * 0.1) * (i as f32 % 3.0 + 1.0))
        .take(64)
        .collect()
}
