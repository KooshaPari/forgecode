'use client';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { createJob } from '@/app/actions/jobs';

interface Model {
  id: string;
  name: string;
  description: string;
  model_type: string;
  input_price_per_token: number;
  output_price_per_token: number;
  providers: {
    id: string;
    name: string;
    region: string;
  };
}

interface CloneJob {
  id: string;
  model_id: string;
  provider_id: string;
  input: string;
  parameters: any;
}

interface CreateJobFormProps {
  userId: string;
  models: Model[];
  cloneJob: CloneJob | null;
}

export default function CreateJobForm({ userId, models, cloneJob }: CreateJobFormProps) {
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [input, setInput] = useState<string>('');
  const [parameters, setParameters] = useState<{
    max_tokens: number;
    temperature: number;
    top_p: number;
    frequency_penalty: number;
    presence_penalty: number;
  }>({
    max_tokens: 100,
    temperature: 0.7,
    top_p: 1,
    frequency_penalty: 0,
    presence_penalty: 0,
  });
  
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [estimatedCost, setEstimatedCost] = useState<number>(0);
  
  const router = useRouter();
  
  // Initialize form with clone job data if available
  useEffect(() => {
    if (cloneJob) {
      setSelectedModel(cloneJob.model_id);
      setInput(cloneJob.input);
      if (cloneJob.parameters) {
        setParameters({
          max_tokens: cloneJob.parameters.max_tokens || 100,
          temperature: cloneJob.parameters.temperature || 0.7,
          top_p: cloneJob.parameters.top_p || 1,
          frequency_penalty: cloneJob.parameters.frequency_penalty || 0,
          presence_penalty: cloneJob.parameters.presence_penalty || 0,
        });
      }
    }
  }, [cloneJob]);
  
  // Calculate estimated cost
  useEffect(() => {
    if (selectedModel && input) {
      const model = models.find(m => m.id === selectedModel);
      if (model) {
        // Estimate input tokens (rough approximation)
        const inputTokens = Math.ceil(input.length / 4);
        
        // Estimate output tokens based on max_tokens
        const outputTokens = parameters.max_tokens;
        
        // Calculate cost
        const inputCost = inputTokens * (model.input_price_per_token || 0);
        const outputCost = outputTokens * (model.output_price_per_token || 0);
        
        setEstimatedCost(inputCost + outputCost);
      }
    }
  }, [selectedModel, input, parameters.max_tokens, models]);
  
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!selectedModel) {
      setError('Please select a model');
      return;
    }
    
    if (!input.trim()) {
      setError('Please enter input text');
      return;
    }
    
    try {
      setError(null);
      setLoading(true);
      
      // Get provider ID from the selected model
      const model = models.find(m => m.id === selectedModel);
      if (!model) {
        throw new Error('Selected model not found');
      }
      
      const result = await createJob({
        userId,
        modelId: selectedModel,
        providerId: model.providers.id,
        input,
        parameters,
      });
      
      if (result.error) {
        setError(result.error);
        return;
      }
      
      // Redirect to job detail page
      router.push(`/jobs/${result.jobId}`);
    } catch (err) {
      console.error('Error creating job:', err);
      setError('An unexpected error occurred');
    } finally {
      setLoading(false);
    }
  };
  
  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {error && (
        <div className="p-4 text-red-700 bg-red-100 rounded-md">
          {error}
        </div>
      )}
      
      <div>
        <label htmlFor="model" className="block text-sm font-medium text-gray-700">
          Model
        </label>
        <select
          id="model"
          name="model"
          value={selectedModel}
          onChange={(e) => setSelectedModel(e.target.value)}
          className="block w-full px-3 py-2 mt-1 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
          required
        >
          <option value="">Select a model</option>
          {models.map((model) => (
            <option key={model.id} value={model.id}>
              {model.name} - {model.providers.name} ({model.providers.region})
            </option>
          ))}
        </select>
        
        {selectedModel && (
          <div className="mt-2">
            <p className="text-sm text-gray-500">
              Input: ${models.find(m => m.id === selectedModel)?.input_price_per_token.toFixed(6)} per token | 
              Output: ${models.find(m => m.id === selectedModel)?.output_price_per_token.toFixed(6)} per token
            </p>
          </div>
        )}
      </div>
      
      <div>
        <label htmlFor="input" className="block text-sm font-medium text-gray-700">
          Input
        </label>
        <textarea
          id="input"
          name="input"
          rows={8}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          className="block w-full px-3 py-2 mt-1 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
          placeholder="Enter your prompt or input text here..."
          required
        />
      </div>
      
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-gray-700">Parameters</h3>
        
        <div>
          <label htmlFor="max_tokens" className="block text-sm font-medium text-gray-700">
            Max Tokens: {parameters.max_tokens}
          </label>
          <input
            type="range"
            id="max_tokens"
            name="max_tokens"
            min="1"
            max="2048"
            step="1"
            value={parameters.max_tokens}
            onChange={(e) => setParameters({ ...parameters, max_tokens: parseInt(e.target.value) })}
            className="block w-full mt-1"
          />
          <p className="mt-1 text-xs text-gray-500">
            The maximum number of tokens to generate in the output.
          </p>
        </div>
        
        <div>
          <label htmlFor="temperature" className="block text-sm font-medium text-gray-700">
            Temperature: {parameters.temperature}
          </label>
          <input
            type="range"
            id="temperature"
            name="temperature"
            min="0"
            max="2"
            step="0.1"
            value={parameters.temperature}
            onChange={(e) => setParameters({ ...parameters, temperature: parseFloat(e.target.value) })}
            className="block w-full mt-1"
          />
          <p className="mt-1 text-xs text-gray-500">
            Controls randomness: 0 is deterministic, higher values are more random.
          </p>
        </div>
        
        <div>
          <label htmlFor="top_p" className="block text-sm font-medium text-gray-700">
            Top P: {parameters.top_p}
          </label>
          <input
            type="range"
            id="top_p"
            name="top_p"
            min="0"
            max="1"
            step="0.05"
            value={parameters.top_p}
            onChange={(e) => setParameters({ ...parameters, top_p: parseFloat(e.target.value) })}
            className="block w-full mt-1"
          />
          <p className="mt-1 text-xs text-gray-500">
            Controls diversity via nucleus sampling: 0.5 means half of all likelihood-weighted options are considered.
          </p>
        </div>
        
        <div>
          <label htmlFor="frequency_penalty" className="block text-sm font-medium text-gray-700">
            Frequency Penalty: {parameters.frequency_penalty}
          </label>
          <input
            type="range"
            id="frequency_penalty"
            name="frequency_penalty"
            min="-2"
            max="2"
            step="0.1"
            value={parameters.frequency_penalty}
            onChange={(e) => setParameters({ ...parameters, frequency_penalty: parseFloat(e.target.value) })}
            className="block w-full mt-1"
          />
          <p className="mt-1 text-xs text-gray-500">
            How much to penalize new tokens based on their existing frequency in the text so far.
          </p>
        </div>
        
        <div>
          <label htmlFor="presence_penalty" className="block text-sm font-medium text-gray-700">
            Presence Penalty: {parameters.presence_penalty}
          </label>
          <input
            type="range"
            id="presence_penalty"
            name="presence_penalty"
            min="-2"
            max="2"
            step="0.1"
            value={parameters.presence_penalty}
            onChange={(e) => setParameters({ ...parameters, presence_penalty: parseFloat(e.target.value) })}
            className="block w-full mt-1"
          />
          <p className="mt-1 text-xs text-gray-500">
            How much to penalize new tokens based on whether they appear in the text so far.
          </p>
        </div>
      </div>
      
      <div className="p-4 bg-gray-100 rounded-md">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-gray-700">Estimated Cost:</span>
          <span className="text-sm font-medium text-gray-900">${estimatedCost.toFixed(6)}</span>
        </div>
        <p className="mt-1 text-xs text-gray-500">
          This is an estimate based on your input length and parameters. Actual cost may vary.
        </p>
      </div>
      
      <div>
        <button
          type="submit"
          disabled={loading}
          className="flex justify-center w-full px-4 py-2 text-sm font-medium text-white bg-indigo-600 border border-transparent rounded-md shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50"
        >
          {loading ? 'Creating...' : 'Create Job'}
        </button>
      </div>
    </form>
  );
}
