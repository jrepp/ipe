# Enterprise scale planning

This is a planning model, not measured savings. The overview uses this model.
The fleet uses separate generated records to test navigation.

## Scope

The user supplied 500 shared policies, 3,000–4,000 consumers, and hundreds to thousands of executions per consumer per quarter.
Consumers include products, components, and product teams.
The model uses 300 / 1,200 / 3,000 executions as low / mode / high assumptions.

A policy definition is a shared rule. An application links a definition to a consumer.
An execution checks an application. A full agent job can make many checks.
Do not multiply execution counts by the number of definitions.

The fleet assigns 100 definitions to each of 3,500 consumers: 350,000 applications.
If all 500 definitions applied to 4,000 consumers, there would be two million applications.
Neither count is a runtime capacity claim.

The salary input is $150,000 per employee per year.
The default 1,000 employees are an editable example, separate from the consumer count.
Their annual salary budget is $150 million.

## Whole agent jobs

Tokens cover a complete job across agents, turns, retries, and repeated context.
They do not describe one request. These starter ranges need workload traces.

| Persona | Mix | Input tokens: low / mode / high | Output tokens: low / mode / high | Total agent seconds: low / mode / high |
| --- | --- | --- | --- | --- |
| Engineering change | 50% | 500k / 3M / 12M | 30k / 200k / 1M | 600 / 1,800 / 7,200 |
| Product plan | 25% | 250k / 1.5M / 6M | 25k / 150k / 600k | 900 / 3,600 / 14,400 |
| Design iteration | 25% | 300k / 2M / 8M | 40k / 200k / 800k | 600 / 2,400 / 10,800 |

Engineering includes source-tree work. Product includes planning and execution.
Design includes exploration and revision. Image charges are excluded from this token model.

## Prices

The baseline uses GPT-6 Astra Standard pricing, checked on November 3, 2026.
Rates per million tokens are $10 input, $1 cached reads, $12.50 cache writes, and $50 output.
Requests above 272k input tokens use twice the input rates and 1.5 times the output rate.
See the [official model prices](https://developers.openai.com/api/docs/models/gpt-6-astra).
The calculator uses the 30-minute cache-write rate. It does not apply Batch, Flex, or Fast rates.

The long-context share estimates the fraction of billed tokens subject to the premium.
Total job context does not itself trigger that premium: the threshold applies per request.
The model applies the same share to input and output. Replace it with billing data when available.

## Assumptions

Each row uses an independent triangular distribution: low / mode / high.
The mode is the most likely input value in this assumed distribution, not the median result.

| Input | Low / mode / high |
| --- | --- |
| Consumers | 3,000 / 3,500 / 4,000 |
| Executions per consumer per quarter | 300 / 1,200 / 3,000 |
| Full governed jobs per execution | 0.02 / 0.08 / 0.20 |
| Preparations per policy per day | 0.1 / 1 / 4 |
| Cached read share | 20% / 60% / 90% |
| Cache write share | 0% / 3% / 10% |
| Long-context share | 0% / 10% / 40% |
| Policy context share of job | 2% / 8% / 20% |
| Reduction within policy context | 20% / 50% / 75% |
| Jobs stopped early by a valid denial | 0% / 1% / 5% |
| Work remaining at that stop | 20% / 50% / 80% |
| Enterprise jobs per employee per calendar day | 5 / 25 / 80 |

Jobs can run across parallel agents. The last row does not count human tasks.
Preparation uses 12,000 input tokens, 900 output tokens, and 18 agent seconds per policy refresh.
Consumer evidence must still be fresh. Shared policy preparation does not remove that requirement.

## Calculation

A quarter is 91.25 days; a year has four quarters.
Daily executions equal consumers times quarterly executions per consumer, divided by 91.25.
Daily governed jobs equal daily executions times the jobs-per-execution ratio.

Price whole jobs using the persona mix and cached, uncached, write, and long-context shares.
For each draw, context reduction equals policy context share times reduction within that share.
Annual context savings equal governed job cost times this reduction, less annual preparation cost.
Preparation scales with 500 shared definitions, not every application.

Early-stop savings apply only to remaining work after context savings.
The overview shows this as a separate scenario. It assumes the denial is correct.
Combined savings add these values within each draw, before finding percentiles.

Enterprise token spend equals employees times jobs per employee per day times job cost times 365.
It is derived from token use; it is not forced above salary spend.
The default modal job costs $22.06. About 18.63 such jobs per employee per day equal the salary budget.

Time savings assume context reduction changes total agent time in the same proportion.
This is an untested approximation. Total agent hours are neither employee hours nor elapsed delivery time.
Token counts include preparation. Costs exclude infrastructure, tools, image charges, and operating costs.

## Results

The calculator uses 20,000 draws and seed 20261103.
P10–P90 contains the middle 80% of these assumed draws.
This is not a measured confidence interval or a calibrated probability of future savings.
Independent draws omit real correlations between job size, cache reuse, retries, and duration.

| Annual result | All inputs at mode | P10 | P50 | P90 |
| --- | ---: | ---: | ---: | ---: |
| Governed jobs without shared context | $29.64M | $31.30M | $77.17M | $167.83M |
| Governed jobs with shared context and preparation | $28.49M | $29.89M | $73.50M | $159.49M |
| Net context savings | $1.16M | $1.05M | $3.34M | $8.87M |
| Separate early-stop savings | $0.14M | $0.16M | $0.61M | $1.86M |
| Combined savings | $1.30M | $1.38M | $4.11M | $10.42M |
| Enterprise token spend | $201.27M | $240.29M | $524.16M | $1,005.92M |

The modal case has 4.2 million executions and 336,000 governed jobs per quarter.
It avoids about 8,732 total agent hours per quarter under the linear time assumption.
Percentiles of differences do not equal differences of independently computed percentiles.

In this sample, 97.405% of draws exceed the salary budget.
In 0.565%, governed jobs exceed total enterprise jobs: those draws have inconsistent scope.
The calculator reports these draws instead of hiding them. No draw has negative net savings with these defaults.
Other inputs can produce negative savings.

## Calibration and reproduction

Collect complete job traces by persona, including retries, output, cached reads, cache writes, and per-request context size.
Measure policy-related context separately. Compare equivalent successful work and check result quality.
Record justified early stops, preparation refreshes, total agent time, and elapsed delivery time.
Add tool, image, infrastructure, and operating costs before claiming financial return.

Run from the repository root:

```sh
node -e 'console.log(JSON.stringify(require("./tools/product-ship-mocks/projection.js").project(), null, 2))'
node --test tools/product-ship-mocks/projection.test.cjs
```

The [calculator](../tools/product-ship-mocks/projection.js) is the executable source of assumptions.
The overview exposes employee count, time window, and token prices for comparison.
