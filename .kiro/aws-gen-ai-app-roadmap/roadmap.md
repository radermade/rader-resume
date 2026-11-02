# GenAI lab — Roadmap

A second Rust binary on the same site, at `https://rader-resume.com/gen-ai`.
You walk a build. The left side is one step: why that product, the code, and
the infrastructure. The right side is a workbench that stays on the screen: a
file tree, an editor, an output pane, and one line for an AWS CLI call. You
write the code and run it. Real CPython runs it, real botocore validates it,
and the transport is intercepted so the response is the one that step would
produce.

This file is the plan. Phase folders with `requirements.md`, `design.md`, and
`tasks.md` start when that phase starts.

## Status

| Phase | Name | Status |
| --- | --- | --- |
| 0 | Decisions | This file |
| 1 | Skeleton on `/gen-ai` | Built locally, not deployed |
| 2 | The workbench is real | Not started |
| 3 | One scenario, start to finish | Not started |
| 4 | The catalog | Not started |

## What a session is

You open a scenario. The brief stays on screen for the whole project. It is
the job: who is asking, what they need back, which systems already exist,
which failure ends the design, and what done means. A sentence that only
names the corpus is not a brief. Under that, the step index shows the whole
build, and which call you are on. Step 1 is the architecture of that system.
From step 2, the lesson is the current call, in this order:

1. Why this product, in this job. What the other choice would get wrong, then the call.
2. The code you type. Python, or an AWS CLI invocation.
3. The infrastructure for that same step. CDK, the same dialect as `infra/`.

Step 1 has no workbench. The diagram is the whole lesson. From step 2 the
workbench is up with the step's files already in it and `boto3` importable.
Next moves the lesson and leaves the workbench alone, so the files you wrote
are still there. A later decision step still has all three parts, so the pane
does not change shape when there is no model call.

`/gen-ai` is a split. The left side is a dark terminal banner that reads
AWS GENAI LAB. The right side is the list of projects. A row opens that
project at step 1. The page does not remember where you were.

## What the sandbox is

Real CPython, compiled to WebAssembly, running in the visitor's own tab. Not a
simulation of Python and not a command matcher. It is not executed on the
Lambda and nothing is sent to AWS: there is no account, no Bedrock spend, no
container behind the route, and no credentials in a browser.

| Input | What answers |
| --- | --- |
| A Python file you wrote | Pyodide runs it. Real `import`, real tracebacks, real line numbers |
| `boto3` in that file | The genuine library, installed from PyPI. It validates parameters and raises its own exceptions. Only the transport is intercepted |
| `aws …` on the command line | A shim that maps the argv onto the same intercepted client, so the CLI form and the Python form reach one place |
| Infrastructure | Read on the left. Nothing in the sandbox applies a stack |

Intercepting at the transport rather than stubbing the library is the whole
point. A missing `maxTokens` raises a real `ParamValidationError`. A call to
the wrong API plane fails as a real `ClientError`. Those exception names and
messages are what the exam asks about, and an authored string cannot teach
them.

The fixture is the teaching output the intercepted transport returns: a
knowledge base id, a written path, a `stopReason`, a citation. A reason is
authored prose, and it is what the lesson adds when the genuine error is
correct but unhelpful.

The filesystem is in-memory and resets on reload. A scenario seeds the files
for its step, so a refresh gives a clean known starting point rather than a
half-edited mystery. Persisting it is a small change if that loss turns out to
hurt, and it is deliberately not the starting point.

The runtime is several megabytes, so it loads on first use of the workbench
rather than on page load. The landing and the lesson prose stay fast, and a
visitor who only reads never pays for it.

## Scenarios are data

A scenario is an authored document: steps, the three panes, the files that
seed the workbench, the intercepted responses, and the reasons the lesson adds
when a genuine error is correct but unhelpful. It is compiled into the binary.
The handler does not read `.kiro/aws-gen-ai` from disk, and a request does not
call a model.

`.kiro/aws-gen-ai` is the source of the reasoning. Writing a scenario is an
authoring task done before deploy. The prose on the step is the decision from
that steering, cut down to the call in front of you.

## Deploy

Same CloudFront distribution and certificate as the resume. Behaviors for
`/gen-ai` and `/gen-ai/*` point at the lab function URL. The default behavior
stays the resume binary. The resume binary does not gain these routes.

The lab behaviors allow POST. The resume's read-only default does not, which
is why `/fit` and `/provenance` already have their own behaviors. Caching is
disabled on the step routes. The landing can be short-cached because it holds
no visitor state.

Runtime `provided.al2023`, `arm64`, cargo-lambda. 128 MB, 3 second timeout.

The Python runtime and the editor are static assets on the lab behavior only,
content-hashed and long-cached. They stay off the resume origin, which keeps
the resume's payload and its policy untouched.

| Route | Method | Role |
| --- | --- | --- |
| `/gen-ai` | GET | The landing list |
| `/gen-ai/{id}` | GET | That project, at step 1 |
| `/gen-ai/{id}` | POST | Advance or go back. Returns the rendered step |
| `/gen-ai/assets/*` | GET | The runtime, the editor, the stubs. Immutable |

There is no run endpoint. Code runs in the visitor's tab, so a command costs
zero invocations and there is no public POST doing work on the server. That is
a smaller attack surface than the alternative, not a larger one.

Advancing a step swaps the lesson pane and leaves the workbench mounted, which
is what keeps the files you wrote. Without script the POST renders the whole
page and the workbench remounts empty, reseeded from the step.

No link from the resume until one scenario runs locally and on the route.
Then one link.

## Guiding principles

- A scenario is a system you could ship and defend. Step 1 is its architecture. After that, a step is one call.
- Step 1 is the diagram, full width, with no workbench. From step 2: why, code, and infrastructure, and the workbench is a column.
- The brief and the step index stay visible. The brief is the job. A step that is only a product name is not a step.
- You learn the call by writing it and running it. Reading the snippet is the
  prompt. The traceback is the practice.
- The Python is real and the library is real. Only the transport is
  intercepted, because a genuine `ParamValidationError` teaches what an
  authored string cannot.
- The sandbox teaches the request shape. It does not pretend to be an account.
  No credentials reach a browser, ever.
- Nothing the visitor writes is stored or sent anywhere. Code runs in their
  own tab, which is why there is no run endpoint to abuse.
- Scenarios are content. A model on the request path is a cost and an abuse
  problem this route does not take on.
- Same deploy path as the resume. A second binary, the same distribution.
- No Step Functions. Serving a page is not a workflow.
- A file tree and an editor, not a shell. `ls` and `cd` are not on the exam,
  and a shell is a dozen builtins of edge cases for a skill nobody is testing.
- The lesson is a dark UI. The landing, the code, the infrastructure, and
  the workbench are the terminal.

## Repo shape

```
crates/
  gen-ai-core/      scenario model, workbench seeds, markup
                    no aws-sdk-* or lambda_* dependencies
  gen-ai-handler/   one HTTP binary, routes under /gen-ai
content/gen-ai/     one document per scenario
assets/gen-ai/      the workbench: runtime loader, editor, the intercept
                    shim, served content-hashed on the lab behavior
.kiro/aws-gen-ai/   the steering each step is cut from
.kiro/aws-gen-ai-app-roadmap/
infra/              the existing CDK app gains the function and behaviors
```

`gen-ai-core` takes a scenario step and returns `maud` markup, including the
files that seed the workbench and the responses the intercept returns. It runs
nothing. The handler does not grow a second copy of that logic, and neither
crate contains a Python interpreter: that is the browser's job.

## Decisions

| Question | Decision |
| --- | --- |
| Where it lives | `rader-resume.com/gen-ai`, same distribution and certificate. A second function URL. The resume binary is unchanged |
| Runtime | `provided.al2023`, `arm64`, cargo-lambda. 128 MB, 3 second timeout |
| A scenario | An end-to-end build. Data path, model call, safety, identity, operations, and at least one failure. As many steps as that build has. No fixed length |
| A step | From step 2: one call. Why, code, infrastructure, the files that seed the workbench, the intercepted response, and the reasons the lesson adds |
| Overview | Step 1 of every scenario is the architecture diagram, and that step has no workbench. Icon above the product name, dashed groups, orthogonal connectors with a short label. The drawing can be wider and taller than a call step |
| Workbench | A column on the right from step 2: file tree, editor with line numbers, output pane, and one line for a CLI call. Not on the overview. Files survive Next. Not a shell: a file tree replaces `cd` and `ls`, which are not on the exam |
| Python | Real CPython via Pyodide, in the visitor's tab. Loaded on first use of the workbench, not on page load |
| The library | Genuine `boto3` and `botocore` from PyPI. Parameter validation and exception classes are theirs. Only the transport is intercepted, so a wrong call fails the way it really fails |
| AWS CLI | A shim mapping argv onto the same intercepted client, so one code path serves both forms. Installing real `awscli` under Pyodide is a spike, not a commitment |
| Editing | Click a file, edit it, run it. A textarea with a line-number gutter, because a traceback names a line and the loop is read-then-fix |
| Filesystem | In-memory, seeded per step, reset on reload. Persisting it is a small later change rather than the starting point |
| Infrastructure | Shown in CDK next to the code. The sandbox does not deploy it |
| Generation | Authored from the steering, embedded at build. No model call on the route |
| Landing | Split screen. Dark ASCII banner, AWS GENAI LAB, on the left. Project names on the right. A row opens that project at step 1. No saved position, no retake |
| Theme | Landing, code, infrastructure, and the workbench are the terminal: black, vivid magenta, vivid cyan. The lesson around them is a dark UI in a sans serif |
| Policy | Pyodide needs `script-src 'self'` and `wasm-unsafe-eval`, which is a real widening of the lab's policy and the reason the runtime is confined to the lab behavior. The resume's policy is untouched |
| Discoverability | No resume link until the route renders a step. Then one link |

## Landing

The front door is a split, and it is dark. The left column is the banner:
ASCII, monospace, AWS GENAI LAB, with a prompt under it. Color is a
terminal on black: the prompt is vivid magenta, the banner and the project
names are vivid cyan. The right column is narrower. It lists the
projects by name. Choosing one opens that project at step 1, which is the
diagram and has no workbench.

Nothing on the landing is a saved position. There is no finished state and
no retake. Walking a project with Next is for that visit. Coming back to the
list and opening the same project starts it again.

## Theme

Two surfaces.

The landing is the terminal. So are the code block, the infrastructure block,
the editor, and the output pane. Black field. The prompt, `radermade@gen-ai:~$`, is vivid
magenta. Commands, code, infrastructure, project names on the landing, and
the AWS and LAB lines of the banner are vivid cyan. GENAI on the banner is
magenta.

The lesson around those blocks is a dark UI. Sans serif. The project name,
the step index, the brief, the reason, the architecture diagram, and
Previous and Next live there. Those labels are not prompts. The terminal is
where something can be typed or read as a command.

The diagram is a light drawing on that dark lesson. Each AWS product is its
architecture icon with the product name under it. Dashed groups mark the
Region and the VPC. Connectors are orthogonal, and each one carries a short
label. The icons are the AWS Architecture Icons for that product, shipped
with the lab. A feature that has no icon of its own, such as Knowledge Bases,
uses the icon of the product it belongs to and is named in full. Step 1 is
that drawing alone.

Nothing on the route uses the resume palette.

## Catalog

The catalog is the set of projects. Phase 3 builds the first at full length.
Phase 4 builds the rest at the same grain. A test in `gen-ai-core` fails the
build when a project here has no document, when a project is missing a stage
from its list, when a project has no `Breaks when` or no `Done means`, or
when a row in the coverage tables has no step.

Every project is a system you would have to design through. The union of the
projects is the Generative AI Developer exam: each task in domains 1 through
5, and each product choice the questions force. After the overview, a step
is one call. The project is why the calls sit together.

A project opens on the job, not on a product. The job names the person
asking, the answer they have to trust, the system that is already running,
and the failure that kills a design. Each step after the diagram says what
you are deciding in that job and which product loses. A row that is only a
service name is not done.

Every project carries a `Breaks when` and a `Done means`, in that order, and
they are not restatements of each other. `Done means` is the acceptance test.
`Breaks when` is the failure that invalidates the architecture rather than
one you fix in the next sprint: a confidentiality incident, a regulatory
finding, a boundary the customer agreed to and you crossed, a blast radius
you collapsed by accident. If the stated failure could be closed as a bug
ticket, it is not the one that ends the design, and the project is not
specified yet.

That pairing is what makes the steps argue. Each step's losing product loses
because choosing it walks toward the `Breaks when`.

Amazon Quick is confirmed and has a project: it is the agentic workspace that
Amazon QuickSight became, GA in October 2025, with Quick Sight, Quick Flows,
Quick Automate, and Quick Research behind one chat surface. Analyst workspace
is that build. Where an older step says QuickSight, it means Quick Sight.

### EU policy assistant

A European manufacturer, about 4,000 people, keeps HR, legal, and security
policies as PDFs. An employee asks "how many weeks of parental leave apply
in Germany," "may I put a customer name in a ticket," or "what does the
security standard say about USB drives." The reply has to name the file and
the section. A confident paraphrase of last year's PDF is a failure even
when the tone is right.

Three departments share one assistant and must not see each other's
documents. HR can read the leave policy. Security cannot. The company will
not send document text, embeddings, or the model call outside the EU. Legal
edits a policy most weeks, so a model trained in January would still be
quoting the old leave rules in March. The assistant is only opened from the
company network. A new prompt does not reach employees until a held-out set
of questions still cites the right file.

Breaks when an answer is fluent, plausible, and sourced from a policy that
was superseded in February. One of those retires the assistant, because
nobody can then tell which of the correct answers were also stale.

Done means an HR employee gets a cited answer from the current HR corpus, a
security employee retrieves nothing from the HR-only documents, and a prompt
that drops citation quality does not publish.

Step 1 is the picture of that system and nothing else. Company network into
Amazon API Gateway, then AWS Lambda in the VPC. Lambda calls Amazon Bedrock
and Bedrock Knowledge Bases. Amazon S3 holds the PDFs. IAM is the Lambda
role. The workbench starts at step 2.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram |
| 2 | Retrieval, not a fine-tune. The policy PDF changes weekly and the answer must cite that file. Training would freeze an old wording |
| 3 | The corpus bucket is in an EU Region, with a lifecycle on the documents. The text is not copied to a bucket you do not control |
| 4 | Parse tables and headings before any chunk exists. A leave table split down the middle cites a number that was never a policy |
| 5 | A managed knowledge base. You are not going to run the index, the ingest workers, and the embedding calls yourself for a policy corpus |
| 6 | Hierarchical chunking, chosen when the data source is created and then locked. A policy section has to stay attached to its heading |
| 7 | One embedding model, Titan Text Embeddings, and the index dimension matches the one it writes. A second model writes vectors the index cannot search, and switching later is a rebuild rather than a setting |
| 8 | Department metadata, and the filter runs before similarity. Finding a "close" security paragraph for an HR question is the failure |
| 9 | Weekly edits use incremental ingest. A deleted policy drops its vectors. A full rebuild every Friday is how you serve Tuesday's PDF on Monday |
| 10 | Retrieve, then Converse. `maxTokens` is set. The model id starts with `eu.` so inference stays in the EU. Converse does not go to `bedrock-agent-runtime` |
| 11 | Citations on the answer. The employee needs the file name, not only a fluent paragraph |
| 12 | A guardrail for grounding and for prompt attack in the retrieved chunks, on a published version. A draft guardrail is not what production calls |
| 13 | IAM for the inference profile and the foundation-model ARN. The guardrail is required on the role. A wildcard model permission is not the design |
| 14 | PrivateLink to `bedrock-runtime`. The call from the VPC does not cross the public internet |
| 15 | A gateway in front. The employee application does not hold Bedrock credentials |
| 16 | The prompt is a versioned template. Editing a string in the Lambda is not how a prompt change is reviewed |
| 17 | Score retrieval and the answer separately. A good paragraph on the wrong file is a retrieval failure, not a model failure |
| 18 | Invocation logs are encrypted. Guardrail trace is off in production. Macie watches the export, because the logs can still hold the raw question |
| 19 | The pipeline blocks the publish when the eval drops. A prompt that cites worse than the last version does not ship because someone approved the words |
| 20 | `ThrottlingException` at a low request rate. The first look is the account quota and the model, not a retry loop |
| 21 | The right document never comes back, or the wrong department's text does. Check the filter, the ingest, and the embedding model before you blame the prompt |

### Product catalog search

A retailer with a few million SKUs. A shopper types "blue waterproof jacket
under 120" or pastes a product id from a receipt. Both have to work in the
same box. The site already runs OpenSearch for keyword search, and the
search team will keep running that cluster. They will not adopt a second
store just because embeddings showed up.

The catalog is two shapes of text. A product page is a short spec sheet. A
buying guide is long. The right jacket is often already in the result list
and sitting in third place. Some questions share no words with the catalog
("something for a wet commute that packs small"). Marketplace sellers paste
near-copies of the same description, and those copies must not fill the
prompt. Prices and availability change through the day on the product bus.
A shopper in the US store must not be offered the EU-only sku.

Breaks when the receipt lookup stops working. A shopper pasting a product id
is the highest-intent traffic on the site, and a search box that fails them
is worse than having added no semantic search at all.

Done means the product id still hits on a keyword, the sentence finds the
jacket even when the wording does not match the title, the EU-only sku is
filtered out before ranking, and a guide that was unpublished this morning
is gone from the index tonight.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. OpenSearch is the store because it is already the search cluster |
| 2 | Hybrid search. The product id is a keyword. The sentence is a vector. A vector-only store fails the receipt lookup |
| 3 | S3 Vectors is the store you would pick with no hybrid requirement and no cluster to run. This catalog has both, so that store is refused |
| 4 | Fixed-size chunks on the short product pages. Semantic chunks on the long guides. One chunking rule would either shred a spec sheet or swallow a guide |
| 5 | Rerank when the right item is in the list and in the wrong place. A new embedding model is not the fix for third place |
| 6 | Query rewrite for a vague question, so the search sees catalog words. HyDE when the question looks nothing like any title |
| 7 | Near-copy pages are deduplicated before they fill the prompt. Five identical seller descriptions are not five pieces of evidence |
| 8 | Event-driven ingest from the product bus. A nightly rebuild serves this morning's discontinued sku |
| 9 | A filter for the store region, applied before similarity. Closeness does not override which store the shopper is in |
| 10 | `overrideSearchType` only because this OpenSearch index supports it. It is not a flag you set on a store that has no keyword side |

### Contract desk

A law firm already runs its matters in Aurora PostgreSQL. Partners and
associates live in that database: client, matter, the clause library, and
the effective date of each signed document. They will not stand up
OpenSearch beside it. The desk has to answer inside the matter they already
have open.

A typical question is not one document. "Does the 2024 master services
agreement for Northwind allow termination for convenience, and does the
statement of work signed in March narrow that?" The answer has to cite the
clause id. A fluent summary with no clause is useless in a markup. The
comparison often needs a third document, the firm's playbook for that deal
type, which says which of the two clauses wins. The playbook is not a
system prompt. It is retrieved, and only the playbook in force for that
client.

The dangerous failure is cross-client. A clause that is semantically close
and belongs to a different client must never be quoted. Effective date
matters the same way: the 2022 agreement is still in the database and is
not the agreement that governs today. Credentials for the database stay in
Secrets Manager. They are not in the Lambda environment as a string someone
copies into a ticket.

Breaks when a clause belonging to another client is quoted into a markup.
That is a confidentiality incident rather than a relevance bug, and it ends
the product instead of degrading it.

Done means an associate in the Northwind matter gets the two governing
clauses and the playbook rule that compares them, each cited, and a query
for a different client returns none of those rows.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. Aurora is the store because it is already the matter database |
| 2 | pgvector on that Aurora database. A second vector store would split the clause from the client and the effective date you already trust |
| 3 | The embedding dimension matches the column. A model change that writes a different width does not search, and it must not be mixed into the old column |
| 4 | Database credentials in Secrets Manager. The rotation is the database's, not a new secret pasted into the prompt |
| 5 | SQL filters for client and effective date run before similarity. "Close" is not a reason to quote another client's contract |
| 6 | A question that needs two documents under a third is decomposed, each part retrieved, then merged. One embedding of the whole question averages three documents into none of them |
| 7 | A customer-managed knowledge base pointed at Aurora. You keep the filters and the SQL. You do not rebuild the matter schema inside a managed index |
| 8 | The knowledge base role, with confused-deputy conditions. The role can read this firm's clauses. It cannot be handed to another account's knowledge base |
| 9 | Create fails once because the role is still propagating. The fix is to wait and retry the create. It is not a different store |

### Case graph

An investigations unit. A matter is a set of relationships, not a pile of
similar PDFs. The investigator asks "which filings did this person sign, and
which of those cite the statute named in the referral." The unit already
records parties, filings, and citations as edges. The text of a filing still
has to be quoted, because the report needs the sentence, not only the node
id.

Similarity search fails this question on purpose. It returns other filings
that sound like the referral and misses the person who signed a filing that
does not resemble the question. The graph walk is the retrieval. The text
chunks sit beside the nodes so the quote is real. A filing from another
matter that shares a party name is the failure mode, so the matter id is a
filter, not a hope.

When they change the embedding model, the old vectors are a different space.
Mixing them makes "nearest" meaningless. The rebuild is part of the model
change, not a later cleanup.

Breaks when the report asserts a relationship the graph cannot defend. An
investigator caught claiming an edge that is not there loses the matter, not
just the answer.

Done means the answer names the signer, the filing, and the statute edge,
quotes the sentence from the filing text, and does not include a
similar-sounding filing from another matter.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. The graph is the retrieval path |
| 2 | Neptune Analytics. A vector store can rank similar filings. It cannot answer who signed what |
| 3 | Graph retrieval beside the text chunks. The edge finds the filing. The chunk supplies the sentence that goes in the report |
| 4 | Metadata for the matter id, applied before the walk returns text. A shared party name is not a shared matter |
| 5 | Citations name the node and the file. A file citation without the relationship hides the path the investigator has to defend |
| 6 | Rebuild the vectors when the embedding model changes. Do not query a mix of two spaces and call the top hit relevant |

### Employee answers

Internal IT is done running pipelines they did not ask for. Staff ask the
company wiki, the SharePoint sites, and the ticket history in their own
words: "how do I get a laptop in the Munich office," "what is the on-call
rotation." Those systems already decide who can open which page. An
engineer must not receive a page their SharePoint group cannot open. HR
must not see another employee's ticket.

The team has refused to own chunking, an embedding job, and a permissions
copy. Building a Bedrock knowledge base would mean re-implementing both.
That is the design this project turns down, and the step is there so you
practice saying why.

Amazon Q Developer is what an engineer uses in the IDE when the question is
about code. It is not the application staff open in the browser. Kendra is
the other managed search product, for a team that wants an index and still
does not want a knowledge-base pipeline.

Breaks when a staff member is shown a page their SharePoint group cannot
open. The permissions are the product, and one leak proves the sources'
access model was never actually in force.

Done means a staff question returns a page that user can already open, a
user without that permission gets nothing from it, and a code question
typed in the IDE never lands in this index.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. The sources stay the system of record for permissions |
| 2 | Amazon Q Business over the connected sources. It applies the permissions those sources already enforce. You do not copy ACLs into metadata you can get wrong |
| 3 | The knowledge base you would otherwise build, and why this system does not. Chunking and a private index are the work IT refused |
| 4 | Kendra when they want a managed search product and still not a knowledge-base pipeline. It is a search index, not a second Q Business |
| 5 | Amazon Q Developer stays in the IDE. A wiki question and a code question are different products, and the console step is there so you index the right one |

### Member services

A membership club. The member is in a chat: "where is order 4412," and
sometimes, in the same thread, "refund it." The order system is a REST API
that already exists. The agent has to look the order up, draft the reply
from that payload and the return policy, and stop when the member asks for
money back. The refund is not a tool the model may complete. A person on the
club's side confirms it, in the club's own process, and the wait expires if
they do not.

This is a new agent. There is no classic agent to preserve. The path is not
a script you can write down: some turns are answered from the order, some
need a second call for shipment status, and a refund turn must not be
"solved" by the lookup tool. The schema has to make those tools hard to
confuse. Arguments are checked before they hit the API. A session that
never resumes is the one where the tool result was never posted back.

Breaks when the model moves money. A refund completed without a person is a
loss and a regulatory problem at once, and no eval score makes it
acceptable afterwards.

Done means the member gets the shipment status from the real order, a
refund request reaches a reviewer and does not move money on the model's
say-so, and an eval fails the run that finished the task with the wrong
tool.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. This is a new agent, so it starts on AgentCore |
| 2 | The steps are not known in advance, so the model chooses the tool. A fixed workflow cannot see "refund" appear halfway through a tracking question |
| 3 | The Converse tool turn. Your code runs the tool. The model does not hold the order API's credentials |
| 4 | A schema specific enough that the model picks the right tool. "Refund" and "where is my order" cannot share a vague `do_order` operation |
| 5 | Arguments validated before the API. A model-invented order id does not get to the refund path |
| 6 | AgentCore Gateway exposes the existing REST API as MCP tools. You do not rewrite the order service |
| 7 | Return of control when the refund must be confirmed in the caller's process. The model proposes. The club's process decides |
| 8 | A Step Functions task token with a timeout, and SNS to the reviewer. A confirmation that never arrives must end the turn |
| 9 | A step cap, a tool timeout, and a definition of done. The agent does not keep calling tools because the reply could be longer |
| 10 | IAM on the tool role is the blast radius. The lookup role cannot call the refund API |
| 11 | Trace while debugging. Trace off once the path is understood, because the trace holds the order |
| 12 | A tool-choice eval. Finishing the task with the wrong tool is a failure even when the sentence reads well |
| 13 | A session that hangs because the tool result was never posted. The fix is to post the result to that session, not to start a new agent |

### Benefits agent

The benefits desk has been live for a year. Employees ask it about
coverage, dependents, and what changed at open enrollment. It is a classic
Bedrock Agent: action groups, an alias, a knowledge base of the plan
documents, a guardrail. Open enrollment is next month. You are adding a
dependent-coverage question. The instruction and the OpenAPI schema both
change.

New systems in this catalog do not start here. This one already did.
Replacing it with AgentCore the week before enrollment is a different
project. The job is to change the live agent without pointing production at
a draft and without letting the model invent a premium.

A premium is a calculation. The model must not do it in its head. An
address change is not something the agent writes into HR. It returns
control. An edit that "does nothing" in the test alias is usually a skipped
prepare, not a bad prompt.

Breaks when production serves a draft. Enrollment runs for three weeks, so a
wrong coverage answer compounds across every employee who read it before
anyone noticed.

Done means the production alias serves the new dependent question, the
draft alias is where you tried it, the premium comes from the code
interpreter, and an address change waits for the HR system.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. Classic agents, because this assistant is already one |
| 2 | An OpenAPI action group and Lambda fulfillment. The plan system already has an API. The agent calls it. It does not browse |
| 3 | `PrepareAgent` after the instruction and the schema change, then the alias moves. The running alias does not see an edit that was only saved |
| 4 | `DRAFT` is not the production alias. Testers use the draft. Employees use the alias you moved |
| 5 | The guardrail id and a numbered version on the agent. "Latest" is not a version you can roll back to |
| 6 | Session memory instead of replaying the whole transcript. The employee already said which plan they are on |
| 7 | The code interpreter for the premium. The model must not arithmetic its way to a benefits number |
| 8 | Return of control for the address change. The HR system of record writes the address. The agent does not |
| 9 | An edit that appears to do nothing because prepare was skipped. Prepare, then ask again, before you rewrite the instruction |

### Desk router

A hospital already runs two assistants. One answers billing. One answers
clinical questions about a protocol. A billing question must not be able to
call the clinical tools, and a protocol question must not see the billing
ledger. Staff hit one desk. Something in front has to send the turn to the
right specialist and no further.

One specialist is a Python loop the team wrote and wants to keep. One tool
is a short eligibility lookup. Another holds a database connection pool and
cannot live in a short Lambda. A new specialist that is only configuration,
not code, can be added without a new service. The old classic agent, if you
move one, will not behave the same and has to be re-tested against the
dialogs you already trust. You do not add a reflection agent on top when
one specialist already answers the question.

Breaks when a billing turn reaches clinical tools. That separation is the
only reason the hospital allowed one desk, and a single crossing withdraws
the permission for all of it.

Done means a billing utterance only reaches billing tools, the Python loop
keeps its session, the pooled tool stays up between calls, and a second
agent is not introduced to "check" an answer that was already good.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. One desk, specialists that do not share tools |
| 2 | Agent Squad, or a supervisor whose specialists stay narrow. One agent with every tool is how a billing question reaches the chart |
| 3 | Strands for the loop you wrote, on AgentCore Runtime, with a real session id. A new session id on every turn forgets the protocol you were in |
| 4 | The MCP lookup on Lambda. It is a short call. It does not need a container |
| 5 | The pooled tool on ECS. A connection pool does not survive a Lambda freeze, and opening a new pool per call is the outage |
| 6 | A Harness only for a new specialist that is config rather than code. The Python loop is not that specialist |
| 7 | Migration off the classic agent is re-tested. It is not behavior-identical. The old dialogs are the test, not a hope |
| 8 | Reflection and an extra agent stay out when one specialist already meets the bar. Another model call is cost and a new way to be wrong |

### Overnight claims

An insurer drops about 50,000 claim packets on the floor every night. A
packet is a scan, and the useful part is often a table: provider, line,
amount. Nobody is waiting on a token. Analysts arrive in the morning and
query the extracted fields. One corrupt file is normal. It must not fail
the other 49,999.

This is not a chat endpoint. Standing up provisioned throughput for a job
that runs once a night and then goes quiet is paying for capacity you do
not use. Parsing happens before the model sees the page, because the model
is not the form reader. The morning question is SQL over the output, not
another model call.

Breaks when one corrupt packet fails the night. Analysts arrive to nothing,
and the backlog doubles before the next window opens.

Done means the night finishes with a table analysts can query, the bad file
is in a dead-letter queue with a reason, and the cost note shows the batch
beating the provisioned estimate.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. A night job, not a chat |
| 2 | Batch inference. Not a chat turn, and not provisioned throughput. The work is a pile of documents with no caller waiting |
| 3 | Textract and advanced parsing before the batch. The model receives the table. It does not invent one from a blurry scan |
| 4 | SQS, a concurrency cap, and a dead-letter queue. The night has a pace. A poison file leaves the queue |
| 5 | SageMaker Processing when the prep is a heavy batch job. A Lambda is the wrong place to unpack 50,000 scans |
| 6 | Glue Data Quality on the tables, then a Glue catalog over the output. A row with no amount does not become a morning surprise |
| 7 | Athena for the morning query. The analysts already know SQL. They do not need a chatbot in front of a finished table |
| 8 | EMR only if the job does not fit Glue. It is not the default because the data is "big" |
| 9 | Partial failure, so one document drops out and the night continues. The default that fails the job is the outage |
| 10 | The cost of the batch written next to a provisioned estimate that loses. The estimate is there so the next person does not "fix" the night by buying throughput |

### Support chat

A consumer support product. The caller is looking at the reply as it
appears. A blank page for four seconds is the bug they file, even when the
final answer is right. Traffic spikes at lunch and is almost quiet at
night. You do not buy a flat reservation for the peak unless an uptime
target forces the commit.

The system prompt and the tool schemas change rarely. The caller's account
changes every turn. Some questions are the public FAQ and can be shared
across callers. "What is the balance on my account" must not be. Easy turns
("what are your hours") can use a small model. A dispute goes to a larger
one, and that choice is configuration, because you will change it without a
deploy of the service. A long transcript starts dropping the middle. You
measure time to first token apart from retrieval time, or you will "speed
up the model" when the wait was the search.

Breaks when a cached answer carries someone else's balance. That is a
disclosure incident, and it is the one failure no latency win justifies.

Done means the caller sees tokens quickly, a cached FAQ never contains
someone's balance, a spike degrades instead of retrying itself into a
throttle, and a context overflow cuts tool output before it cuts the
question.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. The caller is watching the stream |
| 2 | Streaming through API Gateway. A buffered Lambda holds the whole answer and the page stays blank |
| 3 | Prompt cache with the stable prefix first, and the cache point before the user text and the retrieved chunks. The account number is not in the prefix |
| 4 | A timestamp in that prefix, and both cache metrics stay a legitimate zero. The prefix is no longer stable. The fix is to take the timestamp out, not to force a hit |
| 5 | A small model for the easy turn, a larger model when the turn is hard. The model id lives in AppConfig. Changing it is not a deploy |
| 6 | On-demand capacity for the shape of this traffic. Priority when waiting is expensive. Flex when the work can queue. Reserved only if the uptime target requires a commit |
| 7 | `maxTokens` on every call. An unset cap is how one dispute turn becomes the bill |
| 8 | ElastiCache for an exact cache of identical requests. A semantic cache only for the shared FAQs. A semantic hit on a balance question is the incident |
| 9 | CloudFront only for answers that are public and not personalized. A cached account answer is the same incident at the edge |
| 10 | DynamoDB for the session, with old turns summarized. The transcript is not replayed in full on every turn |
| 11 | Lost-in-the-middle on a long transcript. The instruction and the latest turn stay. The middle is what you summarize |
| 12 | Time to first token measured apart from retrieval time. They are different waits and different fixes |
| 13 | A retry storm, then adaptive retry with jitter. `AccessDeniedException` is not retried. Retrying a denied call does not become an allow |
| 14 | The context window, cut in order: tool output, history, chunks, few-shot, and only then a larger model. A bigger model is the last cut, not the first |

### Clinical intake

A clinic. The clinician types a situation and reads the note as it streams,
because they will not wait for a finished paragraph before they know the
assistant heard them. The note must not contain a national identifier, not
even redacted after the fact on screen. An email address can be replaced
with a placeholder, because the rest of the sentence ("contact the patient
at …") still has to make sense. The clinician accepts the note before
anything is written to the chart. Until they accept, the chart does not
change.

An auditor later asks two things: which user invoked the model, and which
chart excerpt a sentence came from. The model must not see the raw
identifier even if a later filter would have caught the output. The logs of
the call still contain the raw body, so "we filtered the answer" is not
"the identifier is gone." This project is not training a model. A labeling
job is a different path and does not gate the chart.

Breaks when the identifier reaches the screen, the log, or the chart. It
cannot be unseen, and a filter applied after the fact is an admission the
requirement was never met.

Done means the identifier never appears in the stream, the email is
anonymized rather than deleted, the chart updates only after a person
accepts, and an auditor can tie the call to a user and a source excerpt.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. The clinician is reading a stream, and a person stands between the note and the chart |
| 2 | Content filters, a denied topic, and word filters for what policy can name. The identifier is not a word you can list |
| 3 | `BLOCK` for the identifier that must not appear at all. `ANONYMIZE` for the email the rest of the note still needs. One action cannot do both jobs |
| 4 | Contextual grounding against the chart excerpts, thresholds started near 0.7. A fluent note that is not in the chart is the failure |
| 5 | `ApplyGuardrail` on the note before any model call. The model never sees the raw identifier. A filter on the output is too late for that requirement |
| 6 | `guardrailConfig` on the chat turn. That is the live call. It is not a second copy of the pre-check |
| 7 | `guardContent` so the trusted system text stays out and the user text, the tool results, and the chunks stay in. Guarding your own instruction throws away the turn |
| 8 | `sync` on the stream. `async` would emit the identifier and would not anonymize it. The clinician would already have read it |
| 9 | Trace off in production. The trace is how the identifier leaves after you blocked it in the answer |
| 10 | Comprehend for entity types in text the pipeline submits. Macie for what is already in the bucket. The guardrail is the inference check. None of the three replaces the others |
| 11 | Masking before the prompt when the model must never see the raw value. Output filtering does not meet that sentence |
| 12 | Invocation logs still hold the raw body. The log group is encrypted with a customer-managed key, readers are few, and retention is set |
| 13 | Secrets Manager for the downstream token. The secret is not in the system prompt, where it would be logged with the call |
| 14 | CloudTrail data events for `InvokeModel`. Management events do not record every call. The auditor's question is a data event |
| 15 | Augmented AI before the system of record. Ground Truth is the labeling path, and it is not this gate |
| 16 | A model card. An eval sliced by the groups you are allowed to measure. SageMaker Clarify when the comparison is a classic model, not a Bedrock chat |
| 17 | Jailbreak and false-refusal sets in the pipeline. A judge model is calibrated on a human sample before it can block a deploy |

### Brand voice

A retailer's product copy has to sound like the brand: short, specific, no
hype words their style guide bans. Prompting holds it for a week and then
drifts. They have a few thousand approved before-and-after pairs from the
copy desk. The voice is stable. The facts are not. This week's price and
whether the item is in stock change constantly, and those must come from
retrieval. Training them into the weights means the model quotes last
Tuesday's price with confidence.

The weights are yours, so you need a registry, an approval, a canary, and a
way back. A full fine-tune is the job when the behavior itself is wrong.
Continued pre-training is the job only when the gap is vocabulary the base
model has never seen. LoRA is the job when a full fine-tune is not
justified by the size of the change. The weekly price list is a knowledge
base update. It is not a reason to train again.

Breaks when the copy states a stale price in the brand's voice. The voice is
what was trained, so a fact trained in beside it is the one mistake another
training run cannot quickly undo.

Done means new copy passes the voice eval, a bad version rolls back by
alias, and a price change never requires a training job.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. Weights for the voice. Retrieval for the facts |
| 2 | Fine-tune on the labeled pairs. Prompting already failed to hold the voice |
| 3 | Continued pre-training only if the need is vocabulary rather than behavior. It does not teach "sound like us" |
| 4 | LoRA when the full fine-tune is not justified. The copy desk's change is a behavior shift of known size, and you price it that way |
| 5 | JumpStart as the starting checkpoint. You are not training from an empty initialization |
| 6 | Ground Truth for the labeled set. The pairs are the asset. A folder of PDFs is not a training set |
| 7 | Register in SageMaker Model Registry, approve, canary, roll back the alias. A weight file on a laptop is not a release |
| 8 | Bedrock custom models are invoked with provisioned throughput. Custom weights do not run on on-demand |
| 9 | A SageMaker endpoint when Bedrock does not host the weights. GPU memory and in-flight tokens are the scale signal, not request count alone |
| 10 | Neo compiles a classic model. It does not speed up a Bedrock call |
| 11 | Model Monitor for drift on that classic endpoint. It is not a dashboard you bolt onto a Bedrock Converse call |
| 12 | An EC2 GPU box only when the OS itself is a requirement. "We want a GPU" is not that requirement |
| 13 | The weekly price list stays in the knowledge base. Retrain when the voice eval fails, not when the price changes |

### Invoice extraction

Accounts payable. One invoice comes in. One JSON object comes out: vendor,
invoice number, line items, total. A person downstream posts that object
into the ledger. A pretty paragraph is not postable. A JSON object that
fails to parse blocks the posting, and a content filter does not repair it.

The business team owns one branch: a vendor-specific exception for a
supplier whose layout they know. They edit that branch. Publishing it is
still a pipeline with an eval and a rollback. Saving it in a canvas is not
the release. Textract reads the form. The model is for the fields the form
reader did not lock, not for re-reading a page you already parsed. The
invoice number is tested with string equality. The memo line is not, because
two correct memos are not character-identical.

Breaks when an unparseable object reaches the posting step. A person is now
reading raw model output to hand-key an invoice, and the automation has cost
more than it saved.

Done means the ledger receives a valid object, a vendor exception the
business team edited does not ship until the eval passes, and a wrong model
body fails with `ValidationException` that you can name.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. A document in, a posted object out |
| 2 | Textract for the form. The model is not the first reader of a structured invoice |
| 3 | Glue Data Quality and a Lambda check before the model sees a batch. A batch of blank scans is not a model problem |
| 4 | Data Wrangler when the prep is tabular and interactive. It is not the production path for the night batch |
| 5 | Comprehend for the vendor entity when the name has to be normalized against the vendor master |
| 6 | A multimodal call when the requirement is one understanding of the page. A pipeline when you need the intermediate text for the ledger and for audit |
| 7 | Low temperature. You want the same invoice number on a second run |
| 8 | A JSON schema, then a parse check. A content filter does not fix a broken object |
| 9 | Few-shot examples in the stable prefix. They are part of the template, not pasted into the user turn where they break the cache |
| 10 | Prompt Flows for the branch the business team owns. CodePipeline for the eval gate and the rollback. The flow is the edit. The pipeline is the publish |
| 11 | Prompt Management approval before the template is published. An unapproved template does not reach the night batch |
| 12 | `ValidationException` from a provider body on the wrong model, and from an assistant prefill the model no longer accepts. The body and the model have to match |
| 13 | String equality is the right test for the extracted invoice number and the wrong test for a paragraph |

### Field photos

Technicians on a job site photograph a failed part. Sometimes they attach a
ten-second clip. The reply has to name the part and the hazard in a sentence
a junior tech can act on: "this is the inlet valve, isolate the line before
you open it." A caption that says "mechanical component" is a miss.

A second pipeline on the same photos only needs a label and a moderation
decision before the image is stored. That job does not need a sentence, and
it is not a reason to call a generative model. An audio note on the clip is
speech. A text model does not hear it. If you later search for similar
failures, you are searching images by meaning, or you are searching the
text of the reply. A text embedding of the photo file retrieves nothing
useful.

Breaks when the hazard is wrong. A junior tech acts on that sentence, so a
confident answer about the wrong valve is an injury rather than a bad
caption.

Done means the tech gets the part and the hazard, the store rejects an
image that fails moderation without a generative call, and a spoken note is
text before a text model sees it.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. One path understands the photo. One path only labels and moderates it |
| 2 | A Claude or Nova image call for the understanding. The tech needs a sentence, not a label list |
| 3 | A Nova model that takes video for the clip. A still-image call on one frame misses the failure that happens across the clip |
| 4 | Rekognition when the pipeline needs labels or moderation and not an answer. Generative understanding is the wrong cost and the wrong output for that gate |
| 5 | Transcribe when the input is audio and the model is text. The text model never receives the audio bytes |
| 6 | The embedding is text unless you are retrieving images by meaning. A text-only embedding model on a photo is the wrong index |

### Contact center

The company already takes calls and chats in Amazon Connect. Most turns are
routine: balance, store hours, a time slot for an appointment. Those stay
in a bot that fills slots and completes the intent. When the caller says
something the bot cannot fill, the turn goes to Bedrock. The caller is
still on the line. A buffered answer is a silence they hang up on.

The chat widget is on the public internet. The agent desktop is a workforce
login, not a consumer account. Supervisors get a small UI in front of the
same API, not a second backend. A contact event that arrives on your
webhook is untrusted until the signature checks.

Breaks when the public widget becomes an open invoke. The chat box is on the
internet, and an unauthenticated path to the model is someone else's bill
and someone else's content, served under your name.

Done means routine intents never reach the model, the hard turn streams,
the public widget is not an open invoke, and a forged contact event does
not start a call.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. Connect is already the front door |
| 2 | Lex for the intent and the slots. The model is the wrong tool for "store hours" and for a slot the bot can fill |
| 3 | The handoff. The hard turn carries the slots already filled. The model does not ask for them again |
| 4 | Streaming on the hard turn. The caller is waiting on tokens |
| 5 | WAF on the public edge. The widget is on the internet. The model endpoint is not |
| 6 | Cognito for the app user. IAM Identity Center for the workforce agent desktop. One identity product does not cover both |
| 7 | Amplify for the supervisor UI in front of the API. The UI is not a reason to put credentials in the browser |
| 8 | A webhook signature check when the contact event is delivered to you. An unsigned event is not a call to act on |

### Plant floor

A factory. The maintenance manuals and the sensor history are not allowed
to leave the site. A technician at the line asks "what does the manual say
before I reset this station." The answer has to come from that manual. A
kiosk on the same line also shows whether the station is stopped. That
status check has to be a fast first byte, and it is not a model call. If
you put the model in front of the kiosk, the status page waits on inference.

Either the manual stays on site and the model runs there, or the site sends
only the prompt it is allowed to send and the model stays in the Region.
Copying the corpus to another Region is a data decision this site is not
making. A function at the CloudFront edge can shape a request. It is not
where the model runs.

Breaks when the manual leaves the site. That boundary is the condition the
plant agreed to, and no retrieval quality argument reopens it.

Done means the status kiosk answers without a model call, the manual never
lands in a bucket outside the site boundary you were given, and a reset
answer cites the on-site manual.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. The status path and the manual path are not the same hop |
| 2 | Outposts when the manual itself cannot leave the site. Otherwise process on site and send only the prompt you are allowed to send |
| 3 | Wavelength for the carrier-edge status check. The kiosk's first byte is a status, not a completion |
| 4 | Lambda@Edge for light request shaping at CloudFront. The model stays in the Region |
| 5 | EBS or EFS only for the volume the on-site container actually needs. The corpus is not "a filesystem problem" by default |
| 6 | Cross-Region replication of the corpus is a separate decision, and this site does not make it |

### CRM notes

Sales notes arrive all day from a CRM the company did not build. A rep
writes "Northwind wants the March discount pulled," and an associate later
asks the assistant what was promised. A one-time backfill is two years of
export files sitting on a file share. After that, the live stream is the
source.

The index has to follow creates, updates, and deletes. A deleted note that
stays in the index is the assistant quoting a promise the rep retracted.
The company already runs Kafka for this bus. A second bus would mean two
lags and two ways to miss a delete. If the note is a row in a table you
own, the change feed is that table's stream. DocumentDB is in the picture
only when the notes already live there. It is not where the vectors go.

Breaks when the assistant quotes a promise the rep retracted. That is a
commitment the company did not make, said in the company's voice, and a
stale delete is how it happens.

Done means a note written at 4pm is searchable soon after, a note deleted
at 4:10 is not quotable at 4:30, and the backfill does not use the live
bus's shape just because both are "getting data in."

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. A live stream and a file backfill are different paths into one index |
| 2 | AppFlow for the SaaS stream. You do not scrape the CRM |
| 3 | DataSync or Transfer Family for the backfill. Two years of files are a transfer, not a stream you replay |
| 4 | MSK because Kafka is already the bus. Kinesis if it were not. You do not add a second bus to hold embeddings |
| 5 | DynamoDB Streams when the note is a row you own. The change feed is the table. You do not poll it |
| 6 | DocumentDB only because the notes already live there. It is not the vector index |
| 7 | Near-real-time embed and upsert. A nightly job quotes yesterday's note as current |
| 8 | A deletion removes the vector. An upsert-only pipeline is how retracted promises survive |
| 9 | Lag between the newest source timestamp and the newest indexed timestamp. That gap is the freshness you can defend |

### Shared platform

A platform team. Six product teams call models through one front door. Each
team has its own prompt, its own budget, and its own idea of shipping on a
Friday. You have to say which team spent what, roll a bad prompt back
without redeploying every service, and hand another account a product they
can launch without inventing their own role.

The console works when you try the call, because the console uses your
user. The application role is a different principal. It is missing
`InvokeModelWithResponseStream`, or it is allowed the foundation model and
not the inference profile the code actually sends. A call from a laptop
hits the public endpoint and succeeds. The same call from the subnet hits
the VPC endpoint policy and fails. The first Anthropic call fails until the
use-case form is accepted. A call to the wrong API plane fails with
`UnknownOperationException`, which is not a bad prompt.

Breaks when one team's loop spends the platform's quota. Every other team is
then down, and the front door that existed to be the control has become the
single point of failure.

Done means a team's spend is visible, a bad prompt rolls back by alias, a
new account launches the approved product, and the four failures above are
ones you can name from the symptom.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. One front door, many teams |
| 2 | The gateway's route table in AppConfig. Parameter Store when the value is a single model id. A route table is not a parameter |
| 3 | WAF and an edge throttle. A team with a loop must not be able to spend the platform's quota |
| 4 | Inference profile tags, then Cost Explorer by team. Untagged calls are spend you cannot assign |
| 5 | Cost Anomaly Detection for the spike. A dashboard you look at on Monday is not a spike detector |
| 6 | A CloudWatch dashboard for tokens, throttles, and latency. Managed Grafana when the view crosses accounts. Quick Sight for the monthly quality and cost report. Three audiences, three tools |
| 7 | Synthetics against the public URL. X-Ray from the edge through the tool. A green canary does not tell you which tool was slow |
| 8 | CodePipeline, CodeBuild, and CodeDeploy with a canary and an alias rollback. The rollback is the alias, not a revert of the repository |
| 9 | CodeArtifact for the shared client library. Service Catalog so another account launches the approved product instead of copying a role |
| 10 | IAM Access Analyzer on the role that grew too wide. Lake Formation when the lake feeding a corpus has column permissions. The Encryption SDK when the application encrypts before S3 |
| 11 | Auto Scaling on the container fleet that has a real scalable dimension. Route 53 and, when the path needs it, Global Accelerator. AppSync only for clients that are already GraphQL. An ELB in front of the ECS tools |
| 12 | The console-versus-app failure. The console uses your user. The app role is missing `InvokeModelWithResponseStream` or the profile ARN |
| 13 | A VPC endpoint policy that works from a laptop and fails from the subnet. The laptop never used the endpoint |
| 14 | Marketplace permission and the Anthropic use-case form on a first call. The form is the missing permission |
| 15 | `UnknownOperationException` from a call on the wrong plane. The operation exists. It exists on the other API |
| 16 | A Well-Architected Generative AI Lens review as the close, against the system you just built |

### Runtime for the toolchain

A media company runs everything on Kubernetes already. The GenAI team has
three things to ship and keeps being told all three belong on the existing
cluster. A stateless HTTP proxy sits in front of the model and has to carry
the same network policy and the same service mesh as the services beside it.
An embedding worker wakes up when a batch of assets lands, runs for twenty
minutes, and then is idle for hours. An internal demo app is one container
with an HTTP port that two people look at, and nobody is on call for it.

The image is the artifact. A tag that moves means an incident where nobody
can say what was running. The platform team will not grant the GenAI team
nodes, and the GenAI team will not take a daemonset they did not ask for.

Breaks when nobody can say which image is serving traffic. A rollback that
cannot name a digest is not a rollback, and the three workloads are on three
runtimes precisely so one bad deploy is not all three at once. A mutable tag
collapses that isolation back into a single blast radius.

Done means the proxy runs beside the existing services under the same
policies, the worker costs nothing while idle, the demo app ships without
anyone provisioning a cluster for it, and a rollback names an exact image.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. Three workloads, three runtimes, one registry |
| 2 | ECR with immutable tags and scan on push. The image digest is what you rolled back to. A moving `latest` is how you cannot say what was running |
| 3 | EKS for the proxy, because this company already runs Kubernetes and the proxy has to inherit the network policy its neighbours have. Not because it is a container |
| 4 | Fargate for the embedding worker. Bursty, no node to manage, nothing to keep warm. EC2 capacity only when a specific instance type is the requirement |
| 5 | ECS when there is no Kubernetes investment to inherit. It is the AWS-native orchestrator, not a lesser EKS |
| 6 | App Runner for the demo app. One container, an HTTP port, no cluster and no on-call. Putting this on EKS is buying a platform to run a demo |
| 7 | Auto Scaling on a dimension that actually scales, and an ELB in front of the proxy. Request count is not the signal when the work is tokens |

### Records archive

A records team at an insurer receives documents all day from brokers, from
scanning stations, and from a partner SFTP drop. Nothing arrives on a
schedule. A document that lands at 09:10 has to be searchable before the
adjuster asks at 09:30, and a nightly job is how it waits until tomorrow.

Retention is decades. Access is unpredictable rather than cold: a file can
be untouched for three years and then read fifty times during a dispute, so
a rule that moves it to archive by age is the rule that makes the dispute
slow. When ingest fails, the team finds out in their channel, not by reading
a dashboard on Monday.

Breaks when a document fails silently. The team learns about it during a
dispute, which is the single moment the archive exists for.

Done means a document is indexed minutes after it lands, a scheduled sweep
reconciles anything the event path missed, storage cost falls without
anyone predicting access, and a failed document reaches the team's channel
with a reason.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. An event path and a scheduled sweep into one index |
| 2 | An EventBridge rule on object-created. The pipeline reacts to arrival. A cron poll is how the 09:10 document is searchable tomorrow |
| 3 | EventBridge Scheduler for the nightly reconciliation. That one is a schedule, and a schedule is not an event you can wait for |
| 4 | EventBridge Pipes where the hop has no logic in it. A Lambda whose whole body forwards a message is a function you now operate |
| 5 | S3 Intelligent-Tiering, because access is unpredictable. Lifecycle is right when the pattern is known by age, which is why the policy corpus uses it and this archive does not |
| 6 | SNS is the topic and Chatbot is the channel. The failure reaches the team where they already are. SES is a different product and not this answer |
| 7 | A dead-letter queue with the reason attached. A document that failed silently is a gap nobody finds until the dispute |
| 8 | Lag between the newest object timestamp and the newest indexed timestamp. That number is the freshness promise you can defend |

### Analyst workspace

An insurer's analytics group has three audiences and keeps buying a fourth
tool for each of them. Business analysts want to ask questions of curated
claims data in their own words, build a chart, and automate the weekly
summary they currently assemble by hand. Data scientists want one place to
reach the lake, the notebooks, and the model work without four consoles and
four sets of credentials. The application engineers maintain the services
around all of it, and their work is code with tests, not a prompt.

The group's instinct is to build a chatbot over the warehouse. They do not
have a team to run one, and the question they actually get is "why did
claims in the north region move last week," which is an analytics question
with a chart attached.

Breaks when an analyst sees a column they are not cleared for. The lake's
permissions are the only reason three surfaces were allowed onto it, so a
workspace that reads past them takes the other two down with it.

Done means an analyst gets that answer and the weekly summary runs without
them, the scientists work in one environment, and the engineers' changes
ship through a repository rather than a saved canvas.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. One governed lake, three surfaces onto it |
| 2 | Amazon Quick for the analyst. Natural-language questions, the chart, and the automation in one workspace. The assistant they were going to build is this product |
| 3 | Quick against Q Business. Q Business answers from enterprise content and honours those sources' permissions. Quick is the analytics and automation workspace over data. The question shape decides, not the word "chat" |
| 4 | Quick Flows and Quick Automate for the weekly summary. A Lambda somebody maintains is the version of this that rots |
| 5 | Quick Sight is where the dashboard lives. It is the BI surface inside Quick, which is what Amazon QuickSight became |
| 6 | SageMaker Unified Studio for the builders. One environment over the lake, the notebooks, and the model work, instead of four consoles and four credential sets |
| 7 | Kiro when the change is application code that needs a spec, a plan, and tests. Amazon Q Developer for inline assistance inside the IDE. Employee answers already drew the line between an IDE assistant and an enterprise index; this is the line between an agentic IDE and an inline one |
| 8 | Lake Formation column permissions under all three surfaces. A workspace that asks nicely is not an access control |

### Credit explanations

A bank already decides credit applications with a scoring system that was
approved, documented, and audited years ago. It is not being replaced. What
the bank wants is an explanation an applicant can read: which factors moved
the decision, in plain language, from the reasons the scorer already
emitted.

The model must not decide. A model that decides is a regulated model on a
different approval path, and nobody is opening that. The explanation still
reaches a customer, so it carries obligations: an adverse-action letter is
reviewed by a person, the wording cannot promise or advise, and a regulator
can later ask which model version produced a specific letter. Fairness is
measured on the scorer, which is a classic model, and separately on the
explanation, which is text and needs a different instrument.

Breaks when the letter states a reason the scorer did not give. That is an
inaccurate adverse action notice, which is a regulatory finding rather than
a quality problem, and it arrives with the bank's signature on it.

Done means the letter explains the existing decision without making one, an
adverse-action explanation is reviewed before it is sent, a fairness result
gates a prompt change, and an auditor can tie one letter to a model version
and the reasons it was given.

| Step | The decision |
| --- | --- |
| 1 | The system, as the architecture diagram. The scorer decides. The model writes. A person stands between an adverse letter and the applicant |
| 2 | The model explains the reasons it was handed. It does not score. Letting it weigh a factor makes it the decision system and changes which approvals apply |
| 3 | A model card with intended use and the limitations stated. "We wrote a prompt" is not a record a regulator accepts |
| 4 | SageMaker Clarify for bias on the scorer, because that is a classic model with features and labels. It is not the instrument for a Converse call |
| 5 | An eval on the explanation, sliced by the attributes you are permitted to measure. Equal quality of explanation is a separate question from equal decisions |
| 6 | Augmented AI before an adverse-action letter is sent. The reviewer is in the process, not a quality sample taken afterwards |
| 7 | Guardrail denied topics for advice and for any promise about a future application. The failure here is a sentence that reads helpfully and commits the bank |
| 8 | Lake Formation column permissions on the feature data behind the reasons. The explanation service reads the reasons, not the applicant's full record |
| 9 | CloudTrail data events and a recorded model version per letter. The auditor's question is which version wrote this, and management events do not answer it |
| 10 | The fairness and quality eval gates the publish. A prompt that explains one group worse does not ship because the sentences improved |

## Coverage

Domains are the exam tasks. Choices are the decisions a question forces you
to make inside those tasks. A step may cover several rows. A row with no
step fails the build.

| Task | Practiced in |
| --- | --- |
| 1.1 Design before the model | Every project opens on the pattern. Policy assistant is the reference |
| 1.2 Select and configure models | Support chat, brand voice, field photos, policy assistant |
| 1.3 Pipelines into the model | Invoice extraction, overnight claims, field photos, contact center, records archive |
| 1.4 Vector store design | Policy assistant, product catalog, contract desk, case graph, employee answers |
| 1.5 Retrieval | Policy assistant, product catalog, contract desk, case graph |
| 1.6 Prompt governance | Policy assistant, invoice extraction, support chat, benefits agent |
| 2.1 Agents and tools | Member services, benefits agent, desk router |
| 2.2 Deployment | Brand voice, overnight claims, support chat, plant floor, desk router, runtime for the toolchain |
| 2.3 Enterprise integration | Policy assistant, CRM notes, contact center, plant floor, shared platform, member services, records archive |
| 2.4 Calling the model | Policy assistant, support chat, invoice extraction, shared platform |
| 2.5 App patterns | Contact center, employee answers, invoice extraction, shared platform, analyst workspace |
| 3.1 Input and output safety | Clinical intake, policy assistant, invoice extraction |
| 3.2 Data security and privacy | Clinical intake, policy assistant, contract desk, shared platform, credit explanations |
| 3.3 Governance and compliance | Clinical intake, policy assistant, shared platform, credit explanations |
| 3.4 Responsible AI | Clinical intake, brand voice, invoice extraction, credit explanations |
| 4.1 Cost | Support chat, overnight claims, brand voice, shared platform, records archive |
| 4.2 Performance | Support chat, brand voice, product catalog |
| 4.3 Monitoring | Shared platform, policy assistant, support chat, records archive |
| 5.1 Evaluation | Policy assistant, clinical intake, member services, invoice extraction, brand voice |
| 5.2 Troubleshooting | Policy assistant, support chat, benefits agent, member services, invoice extraction, shared platform, contract desk |

| Choice | Where you have to make it |
| --- | --- |
| Prompt, retrieval, fine-tune, or an agent | Policy assistant, brand voice, member services |
| Managed knowledge base | Policy assistant |
| OpenSearch, hybrid, rerank | Product catalog |
| S3 Vectors as the simpler store | Product catalog, the step that refuses it |
| pgvector on Aurora | Contract desk |
| Neptune | Case graph |
| Kendra, and Q Business, and Q Developer | Employee answers |
| Amazon Quick against Q Business, and Quick Sight as the BI surface | Analyst workspace |
| Quick Flows and Quick Automate for a recurring task | Analyst workspace |
| SageMaker Unified Studio as the builder's environment | Analyst workspace |
| Kiro against Q Developer: agentic versus inline | Analyst workspace |
| Titan Text Embeddings as the embedding model, and matching the dimension | Policy assistant |
| Hierarchical, semantic, and fixed chunks | Policy assistant, product catalog |
| Query rewrite, HyDE, decomposition, dedupe | Product catalog, contract desk |
| Converse, and InvokeModel when the body is provider-specific | Policy assistant, invoice extraction |
| Retrieve, then generate with citations | Policy assistant |
| `eu.` and `global.` | Policy assistant |
| Cross-Region model ARN | Policy assistant |
| AgentCore Harness, Runtime, Gateway | Desk router, member services |
| Classic agents, prepare, alias | Benefits agent |
| Agent Squad and Strands | Desk router |
| MCP on Lambda, and MCP on ECS | Desk router |
| Step Functions approval, and return of control | Member services, benefits agent |
| Tool-choice eval, and a hung tool turn | Member services |
| Batch, queue, partial failure | Overnight claims |
| EventBridge rules, Scheduler, and Pipes | Records archive |
| ECR, EKS, ECS, Fargate, App Runner | Runtime for the toolchain |
| S3 Intelligent-Tiering against Lifecycle | Records archive, policy assistant |
| SNS with Chatbot for the operator channel | Records archive |
| Provisioned throughput, and when it loses | Overnight claims, brand voice |
| On-demand, Priority, Flex, reserved | Support chat |
| Prompt cache, exact cache, semantic cache, CloudFront | Support chat |
| Streaming, and time to first token | Support chat, contact center |
| AppConfig for the model id | Support chat, shared platform |
| Guardrail filters, grounding, three apply modes, `sync` | Clinical intake, policy assistant |
| Comprehend, Macie, and a guardrail, each on its own job | Clinical intake |
| Augmented AI, Ground Truth, and prompt approval | Clinical intake, brand voice, invoice extraction |
| Model Registry, LoRA, continued pre-training, JumpStart, Neo, Model Monitor | Brand voice |
| SageMaker endpoint versus Bedrock | Brand voice |
| Clarify on a classic model against an eval on generated text | Credit explanations |
| A model that explains against a model that decides | Credit explanations |
| Lake Formation column permissions | Credit explanations, analyst workspace, shared platform |
| Structured output, and Prompt Flows versus CodePipeline | Invoice extraction |
| Multimodal versus Transcribe, Textract, Rekognition | Field photos, invoice extraction |
| Connect and Lex | Contact center |
| Cognito and IAM Identity Center | Contact center, shared platform |
| Outposts, Wavelength, Lambda@Edge | Plant floor |
| AppFlow, DataSync, Transfer Family, Kinesis, MSK, streams | CRM notes |
| Cost Explorer, anomaly detection, tags | Shared platform |
| Grafana, QuickSight, Synthetics, X-Ray | Shared platform |
| WAF, PrivateLink, KMS, Secrets Manager, Lake Formation, Encryption SDK | Shared platform, policy assistant, clinical intake, contract desk |
| Service Catalog, Access Analyzer, CodeArtifact, Well-Architected | Shared platform |
| The error catalog: throttle, access denied, validation, wrong plane, empty retrieve, cache zeros | Policy assistant, support chat, invoice extraction, shared platform, benefits agent |

## Phases

### 1. Skeleton

A function, two behaviors, and the landing split: the banner on the left,
the policy assistant on the right. The row opens the overview diagram. That
step has no workbench. Local `cargo lambda watch` serves `/gen-ai`. The deploy
job builds this binary through the CDK construct.

Done when `https://rader-resume.com/gen-ai` returns that split, the row
opens the overview, and the resume routes are unchanged.

### 2. The workbench is real

Not a step you can type: an environment you can work in. The order is
deliberate, because a step is only worth authoring once there is somewhere to
practice it.

Pyodide loads on first use. The file tree, the editor with a line-number
gutter, and the output pane mount beside the lesson from step 2. A scenario
step seeds files into the in-memory filesystem. Real `boto3` installs from
PyPI, and the transport is intercepted so a call returns the step's response
instead of reaching a network that is not there.

One step is authored against it: retrieve-then-Converse. Running the seeded
file works. Deleting `maxTokens` and running it again raises a genuine
`ParamValidationError` from botocore, not an authored string. A `global.` id
where the job requires `eu.`, and Converse sent to `bedrock-agent-runtime`,
each fail as the real exception with the lesson's reason beside it.

Done when a file written in the editor runs under real CPython, the real
library validates it, each of those three mistakes produces its genuine
exception, and none of it costs a Lambda invocation.

### 3. Policy assistant, end to end

All twenty-one steps, in order, with the job and the step index on the page.
The job stays visible: who is asking, what done means, and the failure that
ends the design. Each step states the decision in that job and seeds the
workbench with the files that step needs. Step 1 is the architecture diagram.
Opening it from the landing starts there. Next changes the lesson and leaves
the workbench mounted, so the files you wrote are still there.

Done when walking from the overview through the empty-retrieval failure shows
the diagram first with no workbench, then a different reason, snippet, and
infrastructure block on each later step. The diagram shows each AWS product as
its icon with the name under it, inside dashed groups, on orthogonal
connectors. The workbench is the same column from step 2 on, and advancing a
step does not clear it.

### 4. The catalog

The other projects, authored at the same grain as the policy assistant. Each
one opens on the job and carries the reason on every step. The landing list
is every name. The coverage test passes.

Done when each project renders its job and every step, the reason is on the
step, every coverage row has a step, and choosing a name on the landing
opens that project at step 1.

## Open questions

1. The browser Python runtime is several megabytes. It is confined to the lab
   behavior and loads on first use of the workbench, so the landing and the
   lesson prose stay fast and a visitor who only reads never pays for it. The
   lab needs its own page-weight budget rather than inheriting the resume's
   100 KB, which was set for a recruiter on a phone.
2. Whether real `awscli` installs under Pyodide. It is pure Python but heavy
   and pulls docutils. The committed design is a shim mapping argv onto the
   same intercepted client, which is honest and small; installing the real
   thing is a spike worth half an hour before phase 2 locks the CLI form.
3. Whether the filesystem should survive a reload. It does not, by decision:
   a step seeds its files, so a refresh is a clean known state. IndexedDB
   would persist it and is a small change if losing an edit turns out to
   annoy more than a mystery half-edited file would.
