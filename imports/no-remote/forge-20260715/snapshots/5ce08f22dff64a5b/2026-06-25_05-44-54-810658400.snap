'use client';

import React, { useState } from 'react';
import { useRouter } from 'next/navigation';
import { createApiKey } from '@/app/actions/api-keys';

interface CreateApiKeyFormProps {
  userId: string;
}

export default function CreateApiKeyForm({ userId }: CreateApiKeyFormProps) {
  const [name, setName] = useState('');
  const [permissions, setPermissions] = useState<string[]>(['read']);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newApiKey, setNewApiKey] = useState<string | null>(null);
  const router = useRouter();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!name) {
      setError('Please enter a name for your API key');
      return;
    }
    
    try {
      setError(null);
      setLoading(true);
      
      const result = await createApiKey(userId, name, permissions);
      
      if (result.error) {
        setError(result.error);
        return;
      }
      
      setNewApiKey(result.key);
      
      // Refresh the page data
      router.refresh();
    } catch (err) {
      console.error('Error creating API key:', err);
      setError('An unexpected error occurred');
    } finally {
      setLoading(false);
    }
  };

  const handlePermissionChange = (permission: string) => {
    setPermissions((prev) => {
      if (prev.includes(permission)) {
        return prev.filter((p) => p !== permission);
      } else {
        return [...prev, permission];
      }
    });
  };

  const copyToClipboard = () => {
    if (newApiKey) {
      navigator.clipboard.writeText(newApiKey);
      alert('API key copied to clipboard');
    }
  };

  if (newApiKey) {
    return (
      <div className="space-y-6">
        <div className="p-4 text-green-700 bg-green-100 rounded-md">
          <p className="font-medium">API key created successfully!</p>
          <p className="mt-2">
            This is the only time your API key will be displayed. Please copy it and store it securely.
          </p>
        </div>
        
        <div className="p-4 border border-gray-300 rounded-md bg-gray-50">
          <div className="flex items-center justify-between">
            <code className="text-sm font-mono break-all">{newApiKey}</code>
            <button
              type="button"
              onClick={copyToClipboard}
              className="p-2 text-indigo-600 hover:text-indigo-900"
              title="Copy to clipboard"
            >
              <svg xmlns="http://www.w3.org/2000/svg" className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
              </svg>
            </button>
          </div>
        </div>
        
        <div className="flex space-x-4">
          <button
            type="button"
            onClick={() => router.push('/api-keys')}
            className="px-4 py-2 text-sm font-medium text-white bg-indigo-600 rounded-md hover:bg-indigo-700"
          >
            Return to API Keys
          </button>
          <button
            type="button"
            onClick={() => {
              setNewApiKey(null);
              setName('');
              setPermissions(['read']);
            }}
            className="px-4 py-2 text-sm font-medium text-indigo-600 bg-white border border-indigo-600 rounded-md hover:bg-indigo-50"
          >
            Create Another API Key
          </button>
        </div>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {error && (
        <div className="p-4 text-red-700 bg-red-100 rounded-md">
          {error}
        </div>
      )}
      
      <div>
        <label htmlFor="name" className="block text-sm font-medium text-gray-700">
          API Key Name
        </label>
        <input
          type="text"
          id="name"
          name="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g., Development, Production, Testing"
          className="block w-full px-3 py-2 mt-1 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
          required
        />
        <p className="mt-1 text-sm text-gray-500">
          Give your API key a descriptive name to help you identify it later.
        </p>
      </div>
      
      <div>
        <label className="block text-sm font-medium text-gray-700">
          Permissions
        </label>
        <div className="mt-2 space-y-2">
          <div className="flex items-center">
            <input
              type="checkbox"
              id="permission-read"
              checked={permissions.includes('read')}
              onChange={() => handlePermissionChange('read')}
              className="w-4 h-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
            />
            <label htmlFor="permission-read" className="block ml-2 text-sm text-gray-900">
              Read (Get models, providers, and job status)
            </label>
          </div>
          
          <div className="flex items-center">
            <input
              type="checkbox"
              id="permission-write"
              checked={permissions.includes('write')}
              onChange={() => handlePermissionChange('write')}
              className="w-4 h-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
            />
            <label htmlFor="permission-write" className="block ml-2 text-sm text-gray-900">
              Write (Create and manage jobs)
            </label>
          </div>
          
          <div className="flex items-center">
            <input
              type="checkbox"
              id="permission-billing"
              checked={permissions.includes('billing')}
              onChange={() => handlePermissionChange('billing')}
              className="w-4 h-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
            />
            <label htmlFor="permission-billing" className="block ml-2 text-sm text-gray-900">
              Billing (Access billing information)
            </label>
          </div>
        </div>
        <p className="mt-1 text-sm text-gray-500">
          Select the permissions you want to grant to this API key.
        </p>
      </div>
      
      <div>
        <button
          type="submit"
          disabled={loading}
          className="flex justify-center w-full px-4 py-2 text-sm font-medium text-white bg-indigo-600 border border-transparent rounded-md shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50"
        >
          {loading ? 'Creating...' : 'Create API Key'}
        </button>
      </div>
    </form>
  );
}
