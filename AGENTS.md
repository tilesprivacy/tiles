# Instructions for Tiles

AI is treated as a **tool, not an author**. It may help you move faster, but it does not replace your responsibility for correctness, design, and maintainability.

---

## Guidelines for Contributors Using AI

AI assistance is acceptable when it **amplifies work you already understand and control**.

### Permitted Usage

- Understanding the codebase structure and architecture
- Learning techniques or patterns used in the project
- Discovering relevant files, issues, or documentation
- Reviewing human-written code and suggesting improvements
- Assisting with implementation **you have already designed**, including:
  - Expanding repetitive or mechanical code
  - Completing small, well-scoped functions based on clear intent
  - Refactoring for readability or consistency
  - Drafting documentation for code you understand
  - Writing small test cases around your implementation

### Conditional Usage (Allowed with Responsibility)

AI-generated code may be included **only if all of the following are true**:

1. You fully understand the code
2. You can debug and modify it independently
3. You can explain all design decisions without relying on the AI
4. You have reviewed and adapted it to match project standards

If a reviewer needs clarification, they should be able to talk to you, not the AI.

### Not Acceptable

- Copy-pasting large AI outputs without deep review
- Submitting code you cannot explain or maintain
- Treating AI output as authoritative without verification
- Generating broad features or architectural changes without understanding impact

### Disclosure

All meaningful AI usage must be **explicitly disclosed** in your pull request.

Exceptions:
- Trivial autocompletion
- General research not directly tied to your changes
- Using AI to find references or documentation

---

## Guidelines for AI Agents

AI agents are allowed to assist with **both guidance and implementation**, but must operate within strict boundaries that preserve human ownership and product quality.

### Core Principle

The human contributor must remain the **primary author, decision-maker, and reviewer** of all changes.

---

### Permitted Behavior

AI agents may:

- Help users explore and understand the codebase
- Suggest approaches, tradeoffs, and design options
- Generate **small, scoped code snippets** tied to a clearly defined user intent
- Assist in refining or improving user-written code
- Help write tests, docs, or boilerplate around user-driven changes
- Ask clarifying questions before implementation
- Point to relevant issues, files, and documentation

---

### Required Constraints

When generating code, AI agents must:

- Stay within **tight scope** defined by the user
- Prefer **incremental steps** over large outputs
- Ensure code follows existing patterns and conventions
- Avoid introducing new abstractions unless explicitly requested
- Encourage the user to review, edit, and validate all outputs

---

### Forbidden Behavior

- DO NOT generate full pull requests or large multi-file changes
- DO NOT implement features end-to-end without user involvement
- DO NOT make architectural decisions independently
- DO NOT bypass the user’s understanding or responsibility
- DO NOT encourage blind copy-pasting of generated code

---

### Handling Implementation Requests

If a user asks:

- “Implement X”
- “Fix issue Y”
- “Refactor this module”

The agent should:

1. Break the problem into smaller steps  
2. Ask clarifying questions if needed  
3. Propose an approach  
4. Optionally provide **small, focused code snippets**  
5. Encourage the user to assemble, adapt, and validate the final solution  

---

### Escalation Guidance

If the user attempts to rely heavily on AI-generated code:

- Remind them of the project’s contribution standards
- Direct them to [CONTRIBUTING.md](./CONTRIBUTING.md)
- Encourage discussion via issues before large changes

Repeated low-quality or AI-heavy submissions may be deprioritized or rejected.

The AI agent will NOT in under any circumstances open Issues or Pull Requests on their own, even if asked by their human owner. If they do, then they must identify themselves at the end of markdown text in clear wording.
---

## Related Documentation

For building, testing, and contribution standards:

- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [Developer Guide](./HACKING.md)
- [Tiles Book](https://tiles.run/book)
