use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use reqwest::StatusCode;
use sentry::capture_error;
use sentry::capture_message;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tracing::*;

#[derive(thiserror::Error, Debug)]
pub enum MatetechError {
    #[error("invalid credentials: {0}")]
    InvalidCredentials(reqwest::Error),
    #[error("forbidden: {0}")]
    Forbidden(reqwest::Error),
    #[error("not found: {0}")]
    NotFound(reqwest::Error),
    #[error("unknown error: {0}")]
    Unknown(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<reqwest::Error> for MatetechError {
    fn from(err: reqwest::Error) -> Self {
        if err.status() == Some(reqwest::StatusCode::FORBIDDEN)
            || err.status() == Some(reqwest::StatusCode::UNAUTHORIZED)
        {
            Self::Forbidden(err)
        } else {
            Self::Other(err.into())
        }
    }
}

const USER_AGENTS: [&str; 16] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/111.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/112.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/109.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/111.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/111.0.0.0 Safari/537.36 Avast/111.0.20716.147",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/112.0.0.0 Safari/537.36 Edg/112.0.1722.34",
    "Mozilla/5.0 (Linux; Android 10; Redmi Note 8) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 11; vivo 1906) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 10; Redmi Note 9S) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 11; SM-A507FN) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 12; 220333QAG) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/105.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 13; SM-A528B) AppleWebKit/537.36 (KHTML, \
     like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; arm_64; Android 11; POCO M2 Pro) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/106.0.0.0 YaBrowser/22.11.7.42.00 SA/3 Mobile \
     Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 16_3_1 like Mac OS X) \
     AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.3 Mobile/15E148 \
     Safari/604.1",
];

async fn build_client() -> Result<reqwest::Client, MatetechError> {
    Ok(reqwest::ClientBuilder::new()
        .user_agent(
            USER_AGENTS
                .choose(&mut rand::thread_rng())
                .expect("there must be at least 1 user agent")
                .to_string(),
        )
        .build()?)
}

#[instrument(skip(password))]
pub async fn login(username: &String, password: &String) -> Result<String, MatetechError> {
    let client = build_client().await?;

    let auth_request = json!({
        "email": username,
        "password": password,
    });

    #[derive(Deserialize)]
    struct AuthResponse {
        data: AuthResponseData,
    }
    #[derive(Deserialize)]
    struct AuthResponseData {
        access_token: String,
    }

    let auth_response = (match client
        .post("https://api.matetech.ru/api/public/companies/3/login")
        .json(&auth_request)
        .send()
        .await?
        .error_for_status()
    {
        Ok(r) => r,
        Err(e) => {
            if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED)
                || e.status() == Some(StatusCode::UNPROCESSABLE_ENTITY)
            {
                return Err(MatetechError::InvalidCredentials(e));
            } else {
                return Err(MatetechError::from(e));
            }
        }
    })
    .json::<AuthResponse>()
    .await?;

    Ok(auth_response.data.access_token)
}

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Solver {
    #[derivative(Debug = "ignore")]
    client: reqwest::Client,
    attempt_id: u32,
    #[derivative(Debug = "ignore")]
    cached_test_result: Option<TestResult>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct GeneratedAnswer {
    /// Question ID
    pub question_id: u32,
    /// Full question to save in database
    pub question: String,
    /// Human-readable answer
    pub human: String,
    /// Full answer to save in database
    pub exact: String,
    /// Machine-readable answer
    pub machine: String,
}

impl Solver {
    pub fn new(token: String, attempt_id: u32) -> Result<Self, MatetechError> {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut auth_value =
            match reqwest::header::HeaderValue::from_str(format!("Bearer {token}").as_str()) {
                Ok(h) => h,
                Err(e) => return Err(MatetechError::Other(e.into())),
            };
        auth_value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);

        let client = reqwest::ClientBuilder::new()
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 \
                 Firefox/112.0",
            )
            .https_only(true)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            attempt_id,
            cached_test_result: None,
        })
    }

    #[instrument]
    pub async fn solve(
        &mut self,
        _speedrun: bool,
    ) -> Result<(String, HashSet<GeneratedAnswer>), MatetechError> {
        let mut full_answer = String::new();
        let mut answers = HashSet::new();

        for (questions_group_id, qustions_group) in self
            .get_test_result()
            .await?
            .data
            .questions
            .into_iter()
            .enumerate()
        {
            for (q_id, question) in qustions_group.iter().enumerate() {
                let answer = self.solve_question(question).await;

                full_answer += &format!(
                    "{}-{}: {}\n",
                    questions_group_id + 1,
                    q_id + 1,
                    answer.human
                );
                answers.insert(answer);
            }
        }

        Ok((full_answer, answers))
    }

    #[instrument(err)]
    async fn get_test_result(&mut self) -> Result<TestResult, MatetechError> {
        Ok(match &self.cached_test_result {
            Some(r) => r.clone(),
            None => {
                let mut test_result = (match self
                    .client
                    .get(format!(
                        "https://api.matetech.ru/api/public/companies/3/test_attempts/{}/result",
                        self.attempt_id
                    ))
                    .send()
                    .await?
                    .error_for_status()
                {
                    Ok(r) => r,
                    Err(e) => {
                        if e.status() == Some(StatusCode::NOT_FOUND) {
                            return Err(MatetechError::NotFound(e));
                        } else {
                            return Err(e.into());
                        }
                    }
                })
                .json::<TestResult>()
                .await?;

                for question_group in &mut test_result.data.questions {
                    for question in question_group {
                        question.answers.sort_by_key(|x| x.sort);
                    }
                }

                self.cached_test_result = Some(test_result.clone());

                test_result
            }
        })
    }

    #[instrument(err)]
    async fn get_question_result(&mut self, qid: u32) -> Result<Question> {
        self.get_test_result()
            .await?
            .data
            .questions
            .into_iter()
            .filter_map(|question_group| question_group.into_iter().find(|q| q.id == qid))
            .next()
            .context("question not found")
    }

    #[instrument(err)]
    async fn get_question(&self, question: u32) -> Result<QuestionInTest, MatetechError> {
        Ok(self.client
            .get(format!(
                "https://api.matetech.ru/api/public/companies/3/test_attempts/{}/question/{question}", self.attempt_id
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<QuestionInTest>()
            .await?)
    }

    #[instrument]
    async fn solve_question(&mut self, question: &Question) -> GeneratedAnswer {
        match match question.r#type.as_str() {
            "radio" => self.solve_radio(question).await,
            "checkbox" => self.solve_checkbox(question).await,
            "input" => self.solve_input(question).await,
            "numeric_input" => self.solve_numeric_input(question).await,
            "sort" => self.solve_sort(question).await,
            "correspondence" => self.solve_correspondence(question).await,
            "text_input" => self.solve_text_input(question).await,
            "select_input" => self.solve_select_input(question).await,
            _ => {
                warn!("unsupported question type: {}", question.r#type);
                capture_message(
                    &format!("unsupported question type: {}", question.r#type),
                    sentry::Level::Warning,
                );
                let unsupported = format!("UNSUPPORTED: {}", question.r#type);
                Ok(GeneratedAnswer {
                    question_id: question.id,
                    question: question.question.clone(),
                    human: unsupported.clone(),
                    exact: unsupported.clone(),
                    machine: unsupported,
                })
            }
        } {
            Ok(answer) => answer,
            Err(e) => {
                error!("error occured during question solving: {}", e);
                capture_error(&e);
                let err = format!("ERROR: {}", e);
                GeneratedAnswer {
                    question_id: question.id,
                    question: question.question.clone(),
                    human: err.clone(),
                    exact: err.clone(),
                    machine: err,
                }
            }
        }
    }

    #[instrument(err)]
    async fn set_answer<T: Serialize + std::fmt::Debug>(
        &mut self,
        question: u32,
        answer: HashMap<u32, T>,
    ) -> Result<(), MatetechError> {
        let question_attempt = self.get_question(question).await?.data.current_attempt.id;

        let set_answer_request = json!({ "answer": answer });

        self.client
            .post(format!(
                "https://api.matetech.ru/api/public/companies/3/question_attempts/{question_attempt}/answer"
            ))
            .json(&set_answer_request)
            .send()
            .await?
            .error_for_status()?;

        self.cached_test_result = None;

        Ok(())
    }

    #[instrument(err)]
    async fn solve_checkbox(
        &mut self,
        question: &Question,
    ) -> Result<GeneratedAnswer, MatetechError> {
        let mut ans_map = HashMap::new();
        for answer in &question.answers {
            ans_map.insert(answer.id, answer.id.to_string());
        }
        self.set_answer(question.id, ans_map).await?;

        let q_result = self.get_question_result(question.id).await?;
        let correct_answers: Vec<(usize, &Answer)> = q_result
            .answers
            .iter()
            .enumerate()
            .filter(|(_, x)| x.user_correct == Some(true))
            .collect();

        let ans_map: HashMap<u32, String> = correct_answers
            .iter()
            .map(|(_, x)| (x.id, x.id.to_string()))
            .collect();
        let human_answer: String = correct_answers
            .iter()
            .map(|(i, _)| (i + 1).to_string())
            .collect();
        let exact_answer: String = correct_answers
            .iter()
            .map(|(_, a)| {
                a.value
                    .clone()
                    .unwrap_or_else(|| String::from("answer value is None"))
                    + "\n"
            })
            .collect();
        let machine_answer: String = correct_answers
            .iter()
            .map(|(_, a)| a.id.to_string() + " ")
            .collect();

        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: human_answer,
            exact: exact_answer,
            machine: machine_answer,
        })
    }

    #[instrument(err)]
    async fn solve_radio(&mut self, question: &Question) -> Result<GeneratedAnswer, MatetechError> {
        for (a_id, answer) in question.answers.iter().enumerate() {
            let mut ans_map = HashMap::new();
            ans_map.insert(answer.id, answer.id.to_string());

            self.set_answer(question.id, ans_map).await?;

            let q_result = self.get_question_result(question.id).await?;
            if q_result
                .answers
                .iter()
                .find(|x| x.id == answer.id)
                .context("must find answer in question")?
                .user_correct
                == Some(true)
            {
                return Ok(GeneratedAnswer {
                    question_id: question.id,
                    question: question.question.clone(),
                    human: (a_id + 1).to_string(),
                    exact: answer
                        .value
                        .clone()
                        .unwrap_or_else(|| String::from("answer value is None")),
                    machine: answer.id.to_string(),
                });
            }
        }

        Err(MatetechError::Unknown(
            "unable to find correct answer".to_string(),
        ))
    }

    #[instrument(err)]
    async fn solve_input(&mut self, question: &Question) -> Result<GeneratedAnswer, MatetechError> {
        if question.answers.len() != 1 {
            return Err(MatetechError::Unknown(
                "there must be exactly 1 answer".to_string(),
            ));
        }

        let value = question.answers[0]
            .value
            .clone()
            .unwrap_or_else(|| String::from("answer value is None"));

        let mut ans_map = HashMap::new();
        ans_map.insert(question.answers[0].id, value.clone());
        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: value.clone(),
            exact: value.clone(),
            machine: value,
        })
    }

    #[instrument(err)]
    async fn solve_numeric_input(
        &mut self,
        question: &Question,
    ) -> Result<GeneratedAnswer, MatetechError> {
        if question.answers.len() != 1 {
            return Err(MatetechError::Unknown(
                "there must be exactly 1 answer".to_string(),
            ));
        }

        let q_result = self.get_question_result(question.id).await?;
        let correct_answer = q_result
            .answers
            .first()
            .context("empty answers")?
            .value
            .clone()
            .unwrap_or_else(|| String::from("answer value is None"));

        let mut ans_map = HashMap::new();
        ans_map.insert(question.answers[0].id, correct_answer.clone());
        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: correct_answer.clone(),
            exact: correct_answer.clone(),
            machine: correct_answer.clone(),
        })
    }

    #[instrument(err)]
    async fn solve_sort(&mut self, question: &Question) -> Result<GeneratedAnswer, MatetechError> {
        let mut ans_map = HashMap::new();
        let mut human_answers = String::new();
        let mut exact_answers = String::new();

        for answer in &question.answers {
            ans_map.insert(
                answer.id,
                answer
                    .correct_position
                    .context("correct_position must be Some(u32)")?
                    .to_string(),
            );
            human_answers += &(answer.id.to_string() + " ");
            exact_answers += &(answer
                .value
                .clone()
                .unwrap_or_else(|| String::from("answer value is None"))
                + "\n");
        }
        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: human_answers.clone(),
            exact: exact_answers.clone(),
            machine: human_answers,
        })
    }

    #[instrument(err)]
    async fn solve_correspondence(
        &mut self,
        question: &Question,
    ) -> Result<GeneratedAnswer, MatetechError> {
        let q_result = self.get_question_result(question.id).await?;
        let correspondence_links = q_result
            .correspondence_links
            .clone()
            .context("no correspondence_links")?;

        let mut ans_map = HashMap::new();
        let human_answers = "DONE".to_string();
        let exact_answers = "DONE".to_string();

        for (id, link) in correspondence_links.into_iter().enumerate() {
            ans_map.insert(id as u32, link);
        }
        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: human_answers.clone(),
            exact: exact_answers.clone(),
            machine: human_answers,
        })
    }

    #[instrument(err)]
    async fn solve_text_input(
        &mut self,
        question: &Question,
    ) -> Result<GeneratedAnswer, MatetechError> {
        let q_result = self.get_question_result(question.id).await?;
        let answers = q_result.answers;

        let mut ans_map = HashMap::new();

        let mut human = String::new();

        for answer in answers {
            let value = answer.value.context("no value")?;

            human.push_str(&value);
            human.push(';');

            ans_map.insert(answer.id, value);
        }

        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: human.clone(),
            exact: human.clone(),
            machine: human,
        })
    }

    #[instrument(err)]
    async fn solve_select_input(
        &mut self,
        question: &Question,
    ) -> Result<GeneratedAnswer, MatetechError> {
        let q_result = self.get_question_result(question.id).await?;
        let select_input_items = q_result
            .select_input_items
            .clone()
            .context("no select_input_items")?;

        let mut ans_map = HashMap::new();
        let mut human = String::new();

        for item in select_input_items {
            if item.correct {
                ans_map.insert(item.answer_id, item.id);
                human.push_str(&item.value);
                human.push(';');
            }
        }
        self.set_answer(question.id, ans_map).await?;

        Ok(GeneratedAnswer {
            question_id: question.id,
            question: question.question.clone(),
            human: human.clone(),
            exact: human.clone(),
            machine: human,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TestResult {
    data: TestResultData,
}

#[derive(Debug, Clone, Deserialize)]
struct TestResultData {
    questions: Vec<Vec<Question>>,
}

#[derive(Debug, Clone, Deserialize)]
struct Question {
    id: u32,
    // sort: Option<u32>,
    question: String,
    r#type: String,
    answers: Vec<Answer>,
    correspondence_links: Option<Vec<CorrespondenceLink>>,
    select_input_items: Option<Vec<SelectInputItem>>,
}

#[derive(Debug, Clone, Deserialize)]
struct Answer {
    id: u32,
    sort: Option<u32>,
    value: Option<String>,
    // full_answer: String,
    // user_answer: Option<String>,
    user_correct: Option<bool>,
    correct_position: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CorrespondenceLink {
    main_id: u32,
    dependent_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct SelectInputItem {
    id: u32,
    answer_id: u32,
    value: String,
    correct: bool,
}

#[derive(Debug, Deserialize)]
struct QuestionInTest {
    data: QuestionInTestData,
}

#[derive(Debug, Deserialize)]
struct QuestionInTestData {
    // question: String,
    // r#type: Option<String>,
    current_attempt: CurrentAttempt,
}

#[derive(Debug, Deserialize)]
struct CurrentAttempt {
    id: u32,
    // points: Option<u32>,
}
