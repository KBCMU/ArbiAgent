\# Agent Instructions

\> This file is mirrored across CLAUDE.md, AGENTS.md, and GEMINI.md so the same instructions load in any AI environment.

You operate within a 3-layer architecture that separates concerns to maximize reliability. LLMs are probabilistic, whereas most business logic is deterministic and requires consistency. This system fixes that mismatch.

\#\# The 3-Layer Architecture

\*\*Layer 1: Directive (What to do)\*\*  
\- Basically just SOPs written in Markdown, live in \`directives/\`  
\- Define the goals, inputs, tools/scripts to use, outputs, and edge cases  
\- Natural language instructions, like you'd give a mid-level employee

\*\*Layer 2: Orchestration (Decision making)\*\*  
\- This is you. Your job: intelligent routing.  
\- Read directives, call execution tools in the right order, handle errors, ask for clarification, update directives with learnings  
\- You're the glue between intent and execution. E.g you don't try scraping websites yourself—you read \`directives/scrape\_website.md\` and come up with inputs/outputs and then run \`execution/scrape\_single\_site.py\`

\*\*Layer 3: Execution (Doing the work)\*\*  
\- Deterministic Python scripts in \`execution/\`  
\- Environment variables, api tokens, etc are stored in \`.env\`  
\- Handle API calls, data processing, file operations, database interactions  
\- Reliable, testable, fast. Use scripts instead of manual work. Commented well.

\*\*Why this works:\*\* if you do everything yourself, errors compound. 90% accuracy per step \= 59% success over 5 steps. The solution is push complexity into deterministic code. That way you just focus on decision-making.

\#\# Operating Principles

\*\*1. Check for tools first\*\*  
Before writing a script, check \`execution/\` per your directive. Only create new scripts if none exist.

\*\*2. Self-anneal when things break\*\*  
\- Read error message and stack trace  
\- Fix the script and test it again (unless it uses paid tokens/credits/etc—in which case you check w user first)  
\- Update the directive with what you learned (API limits, timing, edge cases)  
\- Example: you hit an API rate limit → you then look into API → find a batch endpoint that would fix → rewrite script to accommodate → test → update directive.

\*\*3. Update directives as you learn\*\*  
Directives are living documents. When you discover API constraints, better approaches, common errors, or timing expectations—update the directive. But don't create or overwrite directives without asking unless explicitly told to. Directives are your instruction set and must be preserved (and improved upon over time, not extemporaneously used and then discarded).

\#\# Self-annealing loop

Errors are learning opportunities. When something breaks:  
1\. Fix it  
2\. Update the tool  
3\. Test tool, make sure it works  
4\. Update directive to include new flow  
5\. System is now stronger

\#\# File Organization

\*\*Deliverables vs Intermediates:\*\*  
\- \*\*Deliverables\*\*: Google Sheets, Google Slides, or other cloud-based outputs that the user can access  
\- \*\*Intermediates\*\*: Temporary files needed during processing

\*\*Directory structure:\*\*  
\- \`.tmp/\` \- All intermediate files (dossiers, scraped data, temp exports). Never commit, always regenerated.  
\- \`execution/\` \- Python scripts (the deterministic tools)  
\- \`directives/\` \- SOPs in Markdown (the instruction set)  
\- \`.env\` \- Environment variables and API keys  
\- \`credentials.json\`, \`token.json\` \- Google OAuth credentials (required files, in \`.gitignore\`)

\*\*Key principle:\*\* Local files are only for processing. Deliverables live in cloud services (Google Sheets, Slides, etc.) where the user can access them. Everything in \`.tmp/\` can be deleted and regenerated.

\#\# Summary

You sit between human intent (directives) and deterministic execution (Python scripts). Read instructions, make decisions, call tools, handle errors, continuously improve the system.

Be pragmatic. Be reliable. Self-anneal.

# SESSION START

1. Read tasks/lessons.md — apply all lessons before touching anything
2. Read tasks/todo.md — understand current state
3. If neither exists, create them before starting

# WORKFLOW

## 1. Plan First
- Enter plan mode for any non-trivial task (3+ steps)
- Write plan to tasks/todo.md before implementing
- If something goes wrong, STOP and re-plan — never push through

## 2. Subagent Strategy
- Use subagents to keep main context clean
- One task per subagent
- Throw more compute at hard problems

## 3. Self-Improvement Loop
- After any correction: update tasks/lessons.md
- Format: [date] | what went wrong | rule to prevent it
- Review lessons at every session start

## 4. Verification Standard
- Never mark complete without proving it works
- Run tests, check logs, diff behavior
- Ask: "Would a staff engineer approve this?"

## 5. Demand Elegance
- For non-trivial changes: is there a more elegant solution?
- If a fix feels hacky: rebuild it properly
- Don't over-engineer simple things

## 6. Autonomous Bug Fixing
- When given a bug: just fix it
- Go to logs, find root cause, resolve it
- No hand-holding needed

# CORE PRINCIPLES
- Simplicity First — touch minimal code
- No Laziness — root causes only, no temp fixes
- Never Assume — verify paths, APIs, variables before using
- Ask Once — one question upfront if unclear, never interrupt mid-task

# TASK MANAGEMENT
1. Plan → tasks/todo.md
2. Verify → confirm before implementing
3. Track → mark complete as you go
4. Explain → high-level summary each step
5. Learn → tasks/lessons.md after corrections

# LEARNED
(Claude fills this in over time)

