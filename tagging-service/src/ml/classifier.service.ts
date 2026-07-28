import { promises as fs } from "fs";
import path from "path";
import { ContractsDataAccess } from "../dal/contractsDataAccess.js";

export interface TrainingSample {
  category: string;
  text: string;
}

export interface PredictionResult {
  category: string;
  confidence: number;
}

interface StoredModel {
  algorithm: "tfidf_naive_bayes";
  version: number;
  labels: string[];
  idf: Record<string, number>;
  labelDocCount: Record<string, number>;
  tokenLabelCounts: Record<string, Record<string, number>>;
  vocabularySize: number;
  trainedAt: string | null;
  validationAccuracy: number;
}

const MODEL_PATH = path.resolve(process.cwd(), "src", "ml", "classifier.model.json");

const STOPWORDS = new Set([
  "the",
  "a",
  "an",
  "and",
  "or",
  "to",
  "for",
  "of",
  "in",
  "on",
  "is",
  "are",
  "with",
  "from",
  "by",
  "this",
  "that",
]);

function tokenize(input: string): string[] {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9_ ]+/g, " ")
    .split(/\s+/)
    .map((token) => token.trim())
    .filter((token) => token.length > 1 && !STOPWORDS.has(token));
}

function buildDocumentFrequency(samples: TrainingSample[]): Record<string, number> {
  const df: Record<string, number> = {};
  for (const sample of samples) {
    const seen = new Set(tokenize(sample.text));
    for (const token of seen) {
      df[token] = (df[token] ?? 0) + 1;
    }
  }
  return df;
}

function buildIdf(
  documentFrequency: Record<string, number>,
  totalDocs: number,
): Record<string, number> {
  const idf: Record<string, number> = {};
  for (const [token, docFreq] of Object.entries(documentFrequency)) {
    idf[token] = Math.log((1 + totalDocs) / (1 + docFreq)) + 1;
  }
  return idf;
}

function toTfidf(tokens: string[], idf: Record<string, number>): Record<string, number> {
  const tf: Record<string, number> = {};
  for (const token of tokens) {
    tf[token] = (tf[token] ?? 0) + 1;
  }
  const out: Record<string, number> = {};
  const norm = tokens.length || 1;
  for (const [token, count] of Object.entries(tf)) {
    out[token] = (count / norm) * (idf[token] ?? 1);
  }
  return out;
}

function splitTrainValidation(samples: TrainingSample[]): {
  train: TrainingSample[];
  validation: TrainingSample[];
} {
  const sorted = [...samples].sort((a, b) => a.text.localeCompare(b.text));
  const validationSize = Math.max(1, Math.floor(sorted.length * 0.2));
  return {
    train: sorted.slice(validationSize),
    validation: sorted.slice(0, validationSize),
  };
}

export class ContractClassifierService {
  private model: StoredModel | null = null;

  async loadModel(): Promise<void> {
    const raw = await fs.readFile(MODEL_PATH, "utf8");
    this.model = JSON.parse(raw) as StoredModel;
  }

  async saveModel(model: StoredModel): Promise<void> {
    await fs.writeFile(MODEL_PATH, JSON.stringify(model, null, 2), "utf8");
    this.model = model;
  }

  train(samples: TrainingSample[]): { model: StoredModel; validationAccuracy: number } {
    if (samples.length < 10) {
      throw new Error("At least 10 labeled contracts are required for training");
    }

    const { train, validation } = splitTrainValidation(samples);
    const labels = Array.from(new Set(train.map((sample) => sample.category)));
    const documentFrequency = buildDocumentFrequency(train);
    const idf = buildIdf(documentFrequency, train.length);

    const labelDocCount: Record<string, number> = {};
    const tokenLabelCounts: Record<string, Record<string, number>> = {};
    const vocabulary = new Set<string>();

    for (const label of labels) {
      labelDocCount[label] = 0;
      tokenLabelCounts[label] = {};
    }

    for (const sample of train) {
      labelDocCount[sample.category] = (labelDocCount[sample.category] ?? 0) + 1;
      const tfidf = toTfidf(tokenize(sample.text), idf);
      for (const [token, score] of Object.entries(tfidf)) {
        vocabulary.add(token);
        tokenLabelCounts[sample.category][token] =
          (tokenLabelCounts[sample.category][token] ?? 0) + score;
      }
    }

    const model: StoredModel = {
      algorithm: "tfidf_naive_bayes",
      version: 1,
      labels,
      idf,
      labelDocCount,
      tokenLabelCounts,
      vocabularySize: vocabulary.size,
      trainedAt: new Date().toISOString(),
      validationAccuracy: 0,
    };

    const validationAccuracy = this.evaluateAccuracy(validation, model);
    model.validationAccuracy = validationAccuracy;
    return { model, validationAccuracy };
  }

  predictFromText(text: string): PredictionResult {
    if (!this.model || this.model.labels.length === 0) {
      return { category: "unknown", confidence: 0 };
    }

    const tfidf = toTfidf(tokenize(text), this.model.idf);
    const totalDocs = this.model.labels.reduce(
      (acc, label) => acc + (this.model?.labelDocCount[label] ?? 0),
      0,
    );

    const logScores: Record<string, number> = {};
    for (const label of this.model.labels) {
      const labelDocs = this.model.labelDocCount[label] ?? 0;
      const prior = Math.log((labelDocs + 1) / (totalDocs + this.model.labels.length));
      let score = prior;
      const tokenWeights = this.model.tokenLabelCounts[label] ?? {};
      const tokenTotal = Object.values(tokenWeights).reduce((acc, value) => acc + value, 0);

      for (const [token, tfidfValue] of Object.entries(tfidf)) {
        const tokenWeight = tokenWeights[token] ?? 0;
        const likelihood =
          (tokenWeight + 1) / (tokenTotal + Math.max(this.model.vocabularySize, 1));
        score += tfidfValue * Math.log(likelihood);
      }
      logScores[label] = score;
    }

    const sorted = Object.entries(logScores).sort((a, b) => b[1] - a[1]);
    const [bestLabel, bestScore] = sorted[0];
    const nextScore = sorted[1]?.[1] ?? bestScore - 1;
    const confidence = 1 / (1 + Math.exp(-(bestScore - nextScore)));

    return {
      category: bestLabel,
      confidence: Number(confidence.toFixed(4)),
    };
  }

  predict(input: { code?: string; metadata?: Record<string, unknown> }): PredictionResult {
    const metadata = input.metadata ?? {};
    const metadataText = Object.values(metadata)
      .map((value) => String(value ?? ""))
      .join(" ");
    const text = `${input.code ?? ""} ${metadataText}`.trim();
    return this.predictFromText(text);
  }

  private evaluateAccuracy(samples: TrainingSample[], model: StoredModel): number {
    if (samples.length === 0) {
      return 1;
    }
    const previous = this.model;
    this.model = model;
    let correct = 0;
    for (const sample of samples) {
      const result = this.predictFromText(sample.text);
      if (result.category === sample.category) {
        correct += 1;
      }
    }
    this.model = previous;
    return Number((correct / samples.length).toFixed(4));
  }
}

export function composeTrainingText(input: {
  name?: string;
  description?: string | null;
  code?: string | null;
  tags?: string[];
  metadata?: Record<string, unknown>;
}): string {
  const metadataText = Object.values(input.metadata ?? {})
    .map((value) => String(value ?? ""))
    .join(" ");
  return [
    input.name ?? "",
    input.description ?? "",
    (input.tags ?? []).join(" "),
    input.code ?? "",
    metadataText,
  ]
    .join(" ")
    .trim();
}

export async function retrainAndPersistModel(
  dal: ContractsDataAccess,
  classifier: ContractClassifierService,
): Promise<{ validationAccuracy: number; trainedSamples: number }> {
  const rows = await dal.getCategorizedContractsForTraining(2000);
  const samples = rows.map((row) => ({
    category: row.category,
    text: composeTrainingText({
      name: row.name,
      description: row.description,
      code: row.source_code,
      tags: row.tags,
    }),
  }));
  const { model, validationAccuracy } = classifier.train(samples);
  await classifier.saveModel(model);
  return { validationAccuracy, trainedSamples: samples.length };
}
