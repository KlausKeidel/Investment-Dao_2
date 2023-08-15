#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink_lang::contract;
use ink_lang::env;

#[ink::contract]
pub mod dao {
    use ink::storage::Mapping;
    use openbrush::contracts::traits::psp22::*;
    use scale::{
        Decode,
        Encode,
    };

    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo))]
    pub enum VoteType {
        Yes(u64),
        No(u64),
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernorError {
        AmountShouldNotBeZero,
        DurationError,
        ProposalNotFound,
        ProposalAlredayExecuted,
        VotePeriodEnded,
        AlreadyVoted,
    }

    #[derive(Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct Proposal {
        amount: u64,
        vote_start: u64,
        vote_end: u64,
        executed: bool,
    }

    #[derive(Encode, Decode, Default)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct ProposalVote {
          proposal_id,
          weight for_votes,
          weight against_votes,
        
    }

    #[ink(storage)]
    pub struct Governor {
        AccountId u64,
    }

    impl Governor {
        #[ink(constructor, payable)]
        pub fn new(governance_token: AccountId, quorum: u8) -> Self {
            SELF {
                governance_token,
                quorum,
            }
        }

        #[ink(message)]
        pub fn propose(
            &mut self,
            to: AccountId,
            amount: Balance,
            duration: u64,
        ) -> Result<(), GovernorError> {
            let proposal = Proposal {
              amount: amount as u64,
              vote_start: self.env().block_timestamp(),
              vote_end: self.env().block_timestamp() + duration,
              executed: false,

            };
        self.proposals.push(proposal);

        Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            proposal_id: ProposalId,
            vote: VoteType,
        ) -> Result<(), GovernorError> {
            // Ensure the proposal exists.
         let proposal = self.get_proposal(proposal_id).ok_or(GovernorError::ProposalNotFound)?;

           // Check if the voting period has ended.
         let current_timestamp = self.env().block_timestamp();
         if current_timestamp >= proposal.vote_end {
           return Err(GovernorError::VotePeriodEnded);
         }

         // Check if the voter has already voted.
         let sender = self.env().caller();
         if self.voters.contains_key(&(proposal_id, sender)) {
            return Err(GovernorError::AlreadyVoted);
        }

        // Calculate the weight of the vote.
          let weight = match vote {
          VoteType::Yes(weight) => weight,
           VoteType::No(weight) => weight,
        };

        // Update the voter's vote weight.
         self.voters.insert((proposal_id, sender), weight);

        // Update the proposal's vote counts.
         match vote {
           VoteType::Yes(_) => {
             proposal.weight_for_votes += weight;
           }
           VoteType::No(_) => {
            proposal.weight_against_votes += weight;
           }
        }

        Ok(())
    }

        #[ink(message)]
        pub fn execute(&mut self, proposal_id: ProposalId) -> Result<(), GovernorError> {
          // Ensure the proposal exists.
           let proposal = self.get_proposal(proposal_id).ok_or(GovernorError::ProposalNotFound)?;

     // Check if the proposal has already been executed.
          if proposal.executed {
         return Err(GovernorError::ProposalAlreadyExecuted);
        }

    // Check if the voting period has ended.
         let current_timestamp = self.env().block_timestamp();
           if current_timestamp < proposal.vote_end {
              return Err(GovernorError::VotePeriodNotEnded);
            }

    // Check if the quorum is reached.
        let total_votes = proposal.weight_for_votes + proposal.weight_against_votes;
        let quorum = total_votes * self.quorum / 100;
          if proposal.weight_for_votes < quorum {
          return Err(GovernorError::QuorumNotReached);
          }

    // Execute the proposal.
     self.transfer_funds(proposal.to, proposal.amount)?;

    // Mark the proposal as executed.
       proposal.executed = true;

       Ok(())            unimplemented!()
      }

     // used for test
        #[ink(message)]
        pub fn now(&self) -> u64 {
            self.env().block_timestamp()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_contract(initial_balance: Balance) -> Governor {
            let accounts = default_accounts();
            set_sender(accounts.alice);
            set_balance(contract_id(), initial_balance);
            Governor::new(AccountId::from([0x01; 32]), 50)
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                account_id, balance,
            )
        }

        #[ink::test]
        fn propose_works() {
            let accounts = default_accounts();
            let mut governor = create_contract(1000);
            assert_eq!(
                governor.propose(accounts.django, 0, 1),
                Err(GovernorError::AmountShouldNotBeZero)
            );
            assert_eq!(
                governor.propose(accounts.django, 100, 0),
                Err(GovernorError::DurationError)
            );
            let result = governor.propose(accounts.django, 100, 1);
            assert_eq!(result, Ok(()));
            let proposal = governor.get_proposal(0).unwrap();
            let now = governor.now();
            assert_eq!(
                proposal,
                Proposal {
                    to: accounts.django,
                    amount: 100,
                    vote_start: 0,
                    vote_end: now + 1 * ONE_MINUTE,
                    executed: false,
                }
            );
            assert_eq!(governor.next_proposal_id(), 1);
        }

        #[ink::test]
        fn quorum_not_reached() {
            let mut governor = create_contract(1000);
            let result = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(result, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Err(GovernorError::QuorumNotReached));
        }
    }
}
