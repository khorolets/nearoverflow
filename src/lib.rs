use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, Promise};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const MIN_QUESTION_REWARD: u8 = 10;
const ANSWER_PRICE: u8 = 1;

type Stakes = HashMap<String, u128>;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct Ledger {
    stakes: Stakes,
    questions: HashMap<u32, Question>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct Question {
    content: String,
    reward: u128,
    author_account_id: String,
    answers: Vec<Answer>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct Answer {
    id: u32,
    content: String,
    account_id: String,
    reward: u128,
    is_correct: bool,
}

fn add_stake(stakes: &mut Stakes, account_id: String, amount: u128) {
    *stakes.entry(account_id).or_insert(0) += amount;
}

fn award_answer_author(
    stakes: &mut Stakes,
    stake_holder_id: String,
    answer: &Answer,
    reward: u128,
) -> bool {
    assert!(
        stakes.contains_key(&stake_holder_id),
        "Stake holder has no deposit"
    );
    assert!(
        *stakes.get(&stake_holder_id).unwrap_or(&0u128) >= reward,
        "Stake holder has not enough deposit"
    );

    // Transfer reward
    Promise::new(answer.account_id.clone()).transfer(reward);
    // Decrease stake holder deposit
    *stakes.entry(stake_holder_id).or_insert(0) -= reward;
    true
}

#[near_bindgen]
impl Ledger {
    #[payable]
    pub fn create_question(&mut self, content: String) {
        // We want to ensure account attached enough deposit for the reward
        let attached_deposit = env::attached_deposit();
        assert!(
            attached_deposit >= MIN_QUESTION_REWARD.into(),
            "Min question reward is {}",
            MIN_QUESTION_REWARD
        );

        let sender_id = env::signer_account_id();
        let last_question_id = *self.questions.keys().max().unwrap_or(&0u32);
        self.questions.insert(
            last_question_id + 1,
            Question {
                content,
                reward: attached_deposit,
                author_account_id: sender_id.clone(),
                answers: vec![],
            },
        );
        add_stake(&mut self.stakes, sender_id.clone(), attached_deposit);
    }

    #[payable]
    pub fn create_answer(&mut self, question_id: u32, content: String) {
        assert!(
            self.questions.contains_key(&question_id),
            "Question with id {} not found",
            question_id
        );
        let attached_deposit: u128 = env::attached_deposit();

        assert!(
            attached_deposit >= ANSWER_PRICE.into(),
            "To answer a question you have to pay {}",
            MIN_QUESTION_REWARD
        );
        let sender_id = env::signer_account_id();
        let last_answer_id = self
            .questions
            .get(&question_id)
            .unwrap()
            .answers
            .iter()
            .map(|answer| answer.id)
            .max()
            .unwrap_or(0);
        let answer = Answer {
            id: last_answer_id + 1,
            content,
            account_id: sender_id,
            reward: 0,
            is_correct: false,
        };
        self.questions
            .get_mut(&question_id)
            .unwrap()
            .answers
            .push(answer);
    }

    #[payable]
    pub fn upvote_answer(&mut self, question_id: u32, answer_id: u32) -> &Answer {
        let attached_deposit: u128 = env::attached_deposit();
        assert!(
            attached_deposit > 0,
            "To upvote the answer your deposit have to be greater than 0"
        );

        assert!(
            self.questions.contains_key(&question_id),
            "Question with id {} not found",
            &question_id
        );

        let answer = self
            .questions
            .get_mut(&question_id)
            .unwrap()
            .answers
            .iter_mut()
            .find(|answer| answer.id == answer_id);

        let mut answer = match answer {
            Some(v) => v,
            None => panic!("Answer with id {} not found", &answer_id),
        };

        // transfer deposit to answer author
        Promise::new(answer.account_id.clone()).transfer(attached_deposit);
        answer.reward += attached_deposit;

        answer
    }

    pub fn set_correct_answer<'a>(&'a mut self, question_id: u32, answer_id: u32) -> &'a Answer {
        let signer_id = env::signer_account_id();
        assert!(
            self.questions.contains_key(&question_id),
            "Question with id {} not found",
            &question_id
        );
        assert_eq!(
            &self.questions.get(&question_id).unwrap().author_account_id,
            &signer_id,
            "Signer is not an author of the question and must not select what answer is correct"
        );
        assert!(
            self.questions
                .get(&question_id)
                .unwrap()
                .answers
                .iter()
                .filter(|ans| ans.is_correct == true)
                .count()
                == 0,
            "Correct answer for this question have been selected already"
        );

        let question_reward = self.questions.get(&question_id).unwrap().reward;
        let answer_to_be_correct = self
            .questions
            .get_mut(&question_id)
            .unwrap()
            .answers
            .iter_mut()
            .find(|answer| answer.id == answer_id);

        let mut answer_to_be_correct = match answer_to_be_correct {
            Some(v) => v,
            None => panic!("Answer not found"),
        };
        assert_ne!(
            &answer_to_be_correct.account_id, &signer_id,
            "Question author is not allowed to mark own answer as correct"
        );
        if award_answer_author(
            &mut self.stakes,
            signer_id,
            &answer_to_be_correct,
            question_reward,
        ) {
            answer_to_be_correct.is_correct = true;
            answer_to_be_correct.reward += question_reward;
        } else {
            panic!("Unable to reward the correct answer author");
        }
        answer_to_be_correct
    }

    pub fn list_questions(&self) -> &HashMap<u32, Question> {
        &self.questions
    }
}

/*
 *  TESTS
 */
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, AccountId, VMContext};

    fn alice() -> AccountId {
        "alice".to_string()
    }

    fn bob() -> AccountId {
        "bob".to_string()
    }

    fn robin() -> AccountId {
        "robin".to_string()
    }

    fn get_context(signer_account_id: AccountId, attached_deposit: u128) -> VMContext {
        VMContext {
            current_account_id: "contract_owner".to_string(),
            signer_account_id,
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "alice".to_string(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            epoch_height: 19,
            account_balance: 10,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
        }
    }

    #[test]
    fn alice_can_create_question() {
        let context = get_context(alice(), 10);
        testing_env!(context);
        let mut contract = Ledger {
            stakes: HashMap::new(),
            questions: HashMap::new(),
        };
        contract.create_question("How to I look?".to_string());
        assert_eq!(contract.questions.len(), 1);
        assert_eq!(*contract.stakes.get(&alice()).unwrap_or(&0u128), 10u128);
    }

    #[test]
    fn bob_can_answer_alice_question() {
        let context = get_context(bob(), 1);
        testing_env!(context);
        let mut stakes: HashMap<String, u128> = HashMap::new();
        stakes.insert(alice(), 10);

        let mut questions: HashMap<u32, Question> = HashMap::new();
        questions.insert(
            1,
            Question {
                content: "How do I look?".to_string(),
                reward: 10,
                author_account_id: alice(),
                answers: vec![],
            },
        );
        let mut contract = Ledger { stakes, questions };
        assert_eq!(contract.questions.len(), 1);
        assert_eq!(contract.stakes.get(&"alice".to_string()).unwrap(), &10u128);

        contract.create_answer(1, "You look great!".to_string());

        assert_eq!(contract.questions.get(&1u32).unwrap().answers.len(), 1)
    }

    #[test]
    fn robin_can_upvote_for_existing_question() {
        let context = get_context(robin(), 1);
        testing_env!(context);

        let mut questions: HashMap<u32, Question> = HashMap::new();
        questions.insert(
            1,
            Question {
                content: "How do I look?".to_string(),
                reward: 10,
                author_account_id: alice(),
                answers: vec![Answer {
                    id: 1,
                    content: "Perfect".to_string(),
                    account_id: bob(),
                    reward: 0,
                    is_correct: false,
                }],
            },
        );

        let mut contract = Ledger {
            stakes: HashMap::new(),
            questions,
        };

        let answer = contract.upvote_answer(1, 1);
        assert_eq!(answer.reward, 1);
    }

    #[test]
    fn question_author_chooses_answer_to_be_rewarded() {
        let context = get_context(alice(), 0);
        testing_env!(context);

        let mut questions: HashMap<u32, Question> = HashMap::new();
        questions.insert(
            1,
            Question {
                content: "How do I look?".to_string(),
                reward: 10,
                author_account_id: alice(),
                answers: vec![Answer {
                    id: 1,
                    content: "Perfect".to_string(),
                    account_id: bob(),
                    reward: 0,
                    is_correct: false,
                }],
            },
        );
        let mut stakes: Stakes = HashMap::new();
        stakes.insert(alice().clone(), 10);

        let mut contract = Ledger { stakes, questions };

        contract.set_correct_answer(1, 1);
        assert_eq!(
            contract
                .questions
                .get(&1u32)
                .unwrap()
                .answers
                .first()
                .unwrap()
                .is_correct,
            true
        );
    }
}
