import React from 'react';
import { Metadata } from 'next';
import { createServerComponentClient } from '@supabase/auth-helpers-nextjs';
import { cookies } from 'next/headers';
import { redirect } from 'next/navigation';

export const metadata: Metadata = {
  title: 'Dashboard | LocalBase',
  description: 'Manage your LocalBase account and AI compute resources',
};

export default async function DashboardPage() {
  // Create a Supabase client for server components
  const supabase = createServerComponentClient({ cookies });
  
  // Get the user session
  const { data: { session } } = await supabase.auth.getSession();
  
  // If no session, redirect to sign in
  if (!session) {
    redirect('/signin');
  }
  
  // Get the user profile
  const { data: profile } = await supabase
    .from('profiles')
    .select('*')
    .eq('id', session.user.id)
    .single();
  
  // Get recent jobs
  const { data: recentJobs } = await supabase
    .from('jobs')
    .select('*')
    .eq('user_id', session.user.id)
    .order('created_at', { ascending: false })
    .limit(5);
  
  return (
    <div className="container px-4 py-8 mx-auto">
      <h1 className="text-3xl font-bold">Dashboard</h1>
      
      <div className="grid grid-cols-1 gap-6 mt-8 md:grid-cols-2 lg:grid-cols-3">
        {/* User Info Card */}
        <div className="p-6 bg-white rounded-lg shadow-md">
          <h2 className="text-xl font-semibold">Welcome, {profile?.full_name || session.user.email}</h2>
          <p className="mt-2 text-gray-600">
            {profile?.is_provider ? 'Provider Account' : 'User Account'}
          </p>
        </div>
        
        {/* Usage Stats Card */}
        <div className="p-6 bg-white rounded-lg shadow-md">
          <h2 className="text-xl font-semibold">Usage Statistics</h2>
          <div className="mt-4">
            <p className="text-gray-600">Total Jobs: {recentJobs?.length || 0}</p>
            <p className="text-gray-600">Credits Available: $0.00</p>
          </div>
        </div>
        
        {/* Quick Actions Card */}
        <div className="p-6 bg-white rounded-lg shadow-md">
          <h2 className="text-xl font-semibold">Quick Actions</h2>
          <div className="flex flex-col mt-4 space-y-2">
            <a href="/jobs/new" className="px-4 py-2 text-white bg-indigo-600 rounded-md hover:bg-indigo-700">
              New Job
            </a>
            <a href="/api-keys" className="px-4 py-2 text-white bg-indigo-600 rounded-md hover:bg-indigo-700">
              Manage API Keys
            </a>
            {!profile?.is_provider && (
              <a href="/provider/register" className="px-4 py-2 text-white bg-green-600 rounded-md hover:bg-green-700">
                Become a Provider
              </a>
            )}
          </div>
        </div>
      </div>
      
      {/* Recent Jobs Section */}
      <div className="mt-8">
        <h2 className="text-2xl font-semibold">Recent Jobs</h2>
        
        {recentJobs && recentJobs.length > 0 ? (
          <div className="mt-4 overflow-x-auto">
            <table className="min-w-full bg-white rounded-lg shadow-md">
              <thead className="bg-gray-100">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Job ID
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Model
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Created
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200">
                {recentJobs.map((job) => (
                  <tr key={job.id}>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {job.id.substring(0, 8)}...
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {job.model_id}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm">
                      <span className={`px-2 py-1 rounded-full text-xs ${
                        job.status === 'completed' ? 'bg-green-100 text-green-800' :
                        job.status === 'failed' ? 'bg-red-100 text-red-800' :
                        job.status === 'processing' ? 'bg-blue-100 text-blue-800' :
                        'bg-yellow-100 text-yellow-800'
                      }`}>
                        {job.status}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {new Date(job.created_at).toLocaleString()}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      <a href={`/jobs/${job.id}`} className="text-indigo-600 hover:text-indigo-900">
                        View
                      </a>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="p-6 mt-4 text-center bg-white rounded-lg shadow-md">
            <p className="text-gray-600">No jobs found. Start by creating a new job.</p>
            <a href="/jobs/new" className="inline-block px-4 py-2 mt-4 text-white bg-indigo-600 rounded-md hover:bg-indigo-700">
              Create Job
            </a>
          </div>
        )}
      </div>
    </div>
  );
}
