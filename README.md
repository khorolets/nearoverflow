# nearoverflow

NEAR dApp (Smart Contract) that is somehow replicates StackOverflow functionality.

- Demo of the Contract usage: https://khorolets.github.io/nearoverflow-demo
- Frontend repository: https://github.com/khorolets/nearoverflow-demo

## What does it do?

 * Someone asks a question and pays 10 Ⓝ for that (can be custom amount >= 10 Ⓝ)
 * Question appears in the list
 * Anyone can answer a question (it costs 1 Ⓝ)
 * Original question author can choose what answer is correct (on his/her opinion)
 * Author of the "correct" answer gets question reward (>= 10 Ⓝ) **NB!** Author can't select his own answer to avoid fraud schemes.
 * Anyone can *upvote* the answer for 1 Ⓝ that is immediately transfared to answer author
 * That's all.

---

## Contract methods

### View methods


**`list_questions() -> HashMap<String, Question>`**

Returns JSON object where keys are question IDs and values are Question object.


Response Example:

```javascript
{
    "1": {
        "author_account_id": "account_id_string",
        "content": "Content of the question",
        "reward": 10 000000000000000000000000, // Reward amount (10 Ⓝ)
        "answers": [
            {
                "id": 1, // ID of the answer
                "content": "Content of the answer",
                "account_id": "account_id_of_the_answerer",
                "reward": 0, // Amount of reward that answer already gain
                "is_correct": false // true if this answer is chosen as correct
            },
            ...
        ]
    },
    ...
}
```

---

### Change methods

**`create_question(content: String, reward: u128)`** Must attach >= 10 Ⓝ


Creates question. Creates or increses signer's stake with attached amount. Tokens are hold by the Contract for further transfer to the correct answer (BTW for now we have no mechanism to force fraud author to select correct answer, it's not the point of this example)


**`create_answer(question_id: u32, content)`** Must attach 1 Ⓝ


Adds answer to the question with `question_id`. We still can add answers for any questions even if question alredy has "correct" answer.


**`upvote_answer(question_id: u32, answer_id: u32)`** Must attach 1 Ⓝ

Transfers 1 Ⓝ to the author of the answer with `answer_id` for the question with `question_id`, increases `reward` property for the answer, so everybody can see upvotes.


**`set_correct_answer(question_id: u32, answer_id: u32)`**

May be called only by author of the question with `question_id` (author is the account in `author_account_id` of the question). 

Set `is_correct` to `true` for the answer with `answer_id` in quesion with `question_id`. Transfers question `reward` to answer author, decreases stake for question author.


## Structs used


### Stakes 

Alias of `HashMap<String, u128>` to keep an eye on how many tokens question authors "deposited" to the Contract


### Question

Holds necessary information about the question

 * `content`: String - The question text
 * `reward`: u128 - Amount of tokens that will be transferred to an author of "correct" answer
 * `author_account_id`: String - Account id of the question author
 * `answers`: Vec<Answer> - List of answers added to the question


### Answer

Holds necessary information abut the answer

 * `id`: u32 - Incremental id of the answer for the question (ids can repeat in different questions)
 * `content`: String - The answer text
 * `account_id`: String - Account id of the answer author
 * `reward`: u128 - Amount of tokens were transferred to the answer author (after upvoting or setting answer as "correct") 
 * `is_correct`: bool - Becomes `true` only if question author marked this answer as "correct"


 ### Ledger

 This is the main state of the Contract

 * `stakes`: Stakes - hold the staked (deposits)
 * `questions`: HashMap<u32, Question> - holds all the questions