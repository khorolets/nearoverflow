use std::collections::HashMap;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use near_sdk::{env, near_bindgen};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const MIN_QUESTION_REWARD: u8 = 10;
const ANSWER_PRICE: u8 = 1;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct Ledger {
    stakes: HashMap<String, u128>,
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
    is_correct: bool,
}

#[near_bindgen]
impl Ledger {
    fn add_stake(&mut self, account_id: String, amount: u128) {
        *self.stakes.entry(account_id).or_insert(0) += amount;
    }

    pub fn create_question(&mut self, content: String) {
        // We want to ensure 
        let attached_deposit = env::attached_deposit();
        assert!(attached_deposit >= MIN_QUESTION_REWARD.into(), "Min question reward is {}", MIN_QUESTION_REWARD);

        let sender_id = env::signer_account_id();
        let last_question_id= *self.questions.keys().max().unwrap_or(&0u32);
        self.questions.insert(
            last_question_id + 1, 
            Question {
                content, 
                reward: attached_deposit, 
                author_account_id: sender_id.clone(), 
                answers: vec![],
            }
        );
        self.add_stake(sender_id.clone(), attached_deposit);
    }

    pub fn create_answer(&mut self, question_id: u32, content: String) {
        assert!(self.questions.contains_key(&question_id), "Question with id {} not found", question_id);
        let attached_deposit: u128 = env::attached_deposit();
        
        assert!(attached_deposit >= ANSWER_PRICE.into(), "To answer a question you have to pay {}", MIN_QUESTION_REWARD);
        
        let sender_id = env::signer_account_id();
        let last_answer_id = self.questions
            .get(&question_id)
            .unwrap()
            .answers
            .iter()
            .map(
                |answer| answer.id
            )
            .max()
            .unwrap_or(0);
        let answer = Answer { 
            id: last_answer_id + 1, 
            content, 
            account_id: sender_id, 
            is_correct: false 
        };
        self.questions.get_mut(&question_id).unwrap().answers.push(answer);
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
            account_balance: 0,
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
        let mut contract = Ledger { stakes: HashMap::new(), questions: HashMap::new() };
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
            }
        );
        let mut contract = Ledger { stakes, questions };
        assert_eq!(contract.questions.len(), 1);
        assert_eq!(contract.stakes.get(&"alice".to_string()).unwrap(), &10u128);

        contract.create_answer(1, "You look great!".to_string());

        assert_eq!(contract.questions.get(&1u32).unwrap().answers.len(), 1)
    }
}