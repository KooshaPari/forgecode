import { 
  signUp, 
  signIn, 
  signOut, 
  resetPassword, 
  updatePassword,
  getCurrentUser,
  getUserProfile,
  updateUserProfile,
  generateApiKey,
  listApiKeys,
  deleteApiKey,
  createJob,
  listJobs,
  getJobDetails
} from '@/lib/supabase';
import { supabase } from '@/lib/supabase';

// Mock the supabase client
jest.mock('@/lib/supabase', () => {
  const mockSupabase = {
    auth: {
      signUp: jest.fn(),
      signInWithPassword: jest.fn(),
      signOut: jest.fn(),
      resetPasswordForEmail: jest.fn(),
      updateUser: jest.fn(),
      getUser: jest.fn(),
    },
    from: jest.fn().mockReturnThis(),
    select: jest.fn().mockReturnThis(),
    insert: jest.fn().mockReturnThis(),
    update: jest.fn().mockReturnThis(),
    delete: jest.fn().mockReturnThis(),
    eq: jest.fn().mockReturnThis(),
    single: jest.fn(),
    order: jest.fn().mockReturnThis(),
  };
  
  return {
    supabase: mockSupabase,
    signUp: jest.requireActual('@/lib/supabase').signUp,
    signIn: jest.requireActual('@/lib/supabase').signIn,
    signOut: jest.requireActual('@/lib/supabase').signOut,
    resetPassword: jest.requireActual('@/lib/supabase').resetPassword,
    updatePassword: jest.requireActual('@/lib/supabase').updatePassword,
    getCurrentUser: jest.requireActual('@/lib/supabase').getCurrentUser,
    getUserProfile: jest.requireActual('@/lib/supabase').getUserProfile,
    updateUserProfile: jest.requireActual('@/lib/supabase').updateUserProfile,
    generateApiKey: jest.requireActual('@/lib/supabase').generateApiKey,
    listApiKeys: jest.requireActual('@/lib/supabase').listApiKeys,
    deleteApiKey: jest.requireActual('@/lib/supabase').deleteApiKey,
    createJob: jest.requireActual('@/lib/supabase').createJob,
    listJobs: jest.requireActual('@/lib/supabase').listJobs,
    getJobDetails: jest.requireActual('@/lib/supabase').getJobDetails,
  };
});

describe('Supabase Client', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });
  
  describe('Authentication Functions', () => {
    it('signUp calls supabase.auth.signUp with correct parameters', async () => {
      const mockResponse = { data: { user: { id: '123' } }, error: null };
      supabase.auth.signUp.mockResolvedValueOnce(mockResponse);
      
      const result = await signUp('test@example.com', 'password123');
      
      expect(supabase.auth.signUp).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'password123',
      });
      
      expect(result).toEqual(mockResponse);
    });
    
    it('signIn calls supabase.auth.signInWithPassword with correct parameters', async () => {
      const mockResponse = { data: { user: { id: '123' } }, error: null };
      supabase.auth.signInWithPassword.mockResolvedValueOnce(mockResponse);
      
      const result = await signIn('test@example.com', 'password123');
      
      expect(supabase.auth.signInWithPassword).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'password123',
      });
      
      expect(result).toEqual(mockResponse);
    });
    
    it('signOut calls supabase.auth.signOut', async () => {
      const mockResponse = { error: null };
      supabase.auth.signOut.mockResolvedValueOnce(mockResponse);
      
      const result = await signOut();
      
      expect(supabase.auth.signOut).toHaveBeenCalled();
      expect(result).toEqual(mockResponse);
    });
    
    it('resetPassword calls supabase.auth.resetPasswordForEmail with correct parameters', async () => {
      const mockResponse = { data: {}, error: null };
      supabase.auth.resetPasswordForEmail.mockResolvedValueOnce(mockResponse);
      
      // Mock window.location.origin
      Object.defineProperty(window, 'location', {
        value: {
          origin: 'http://localhost:3000',
        },
        writable: true,
      });
      
      const result = await resetPassword('test@example.com');
      
      expect(supabase.auth.resetPasswordForEmail).toHaveBeenCalledWith(
        'test@example.com',
        { redirectTo: 'http://localhost:3000/reset-password' }
      );
      
      expect(result).toEqual(mockResponse);
    });
    
    it('updatePassword calls supabase.auth.updateUser with correct parameters', async () => {
      const mockResponse = { data: { user: { id: '123' } }, error: null };
      supabase.auth.updateUser.mockResolvedValueOnce(mockResponse);
      
      const result = await updatePassword('newpassword123');
      
      expect(supabase.auth.updateUser).toHaveBeenCalledWith({
        password: 'newpassword123',
      });
      
      expect(result).toEqual(mockResponse);
    });
  });
  
  describe('User Functions', () => {
    it('getCurrentUser calls supabase.auth.getUser', async () => {
      const mockUser = { id: '123', email: 'test@example.com' };
      supabase.auth.getUser.mockResolvedValueOnce({
        data: { user: mockUser },
      });
      
      const result = await getCurrentUser();
      
      expect(supabase.auth.getUser).toHaveBeenCalled();
      expect(result).toEqual(mockUser);
    });
    
    it('getUserProfile calls supabase.from("profiles") with correct parameters', async () => {
      const mockProfile = { id: '123', name: 'Test User' };
      supabase.from.mockReturnValueOnce({
        select: jest.fn().mockReturnThis(),
        eq: jest.fn().mockReturnThis(),
        single: jest.fn().mockResolvedValueOnce({
          data: mockProfile,
          error: null,
        }),
      });
      
      const result = await getUserProfile('123');
      
      expect(supabase.from).toHaveBeenCalledWith('profiles');
      expect(result).toEqual({ data: mockProfile, error: null });
    });
    
    it('updateUserProfile calls supabase.from("profiles") with correct parameters', async () => {
      const mockResponse = { data: { id: '123', name: 'Updated Name' }, error: null };
      supabase.from.mockReturnValueOnce({
        update: jest.fn().mockReturnThis(),
        eq: jest.fn().mockResolvedValueOnce(mockResponse),
      });
      
      const updates = { name: 'Updated Name' };
      const result = await updateUserProfile('123', updates);
      
      expect(supabase.from).toHaveBeenCalledWith('profiles');
      expect(result).toEqual(mockResponse);
    });
  });
  
  describe('API Key Functions', () => {
    it('generateApiKey calls supabase.from("api_keys") with correct parameters', async () => {
      // Mock crypto.randomUUID
      const mockUUID = '123e4567-e89b-12d3-a456-426614174000';
      global.crypto = {
        ...global.crypto,
        randomUUID: jest.fn().mockReturnValue(mockUUID),
      };
      
      const mockApiKey = { 
        id: 'key123', 
        name: 'Test API Key',
        key: `lb_${mockUUID.replace(/-/g, '')}`,
      };
      
      supabase.from.mockReturnValueOnce({
        insert: jest.fn().mockReturnThis(),
        select: jest.fn().mockReturnThis(),
        single: jest.fn().mockResolvedValueOnce({
          data: mockApiKey,
          error: null,
        }),
      });
      
      const result = await generateApiKey('user123', 'Test API Key', ['read:models']);
      
      expect(supabase.from).toHaveBeenCalledWith('api_keys');
      expect(result).toEqual({ data: mockApiKey, error: null });
    });
    
    it('listApiKeys calls supabase.from("api_keys") with correct parameters', async () => {
      const mockApiKeys = [
        { id: 'key1', name: 'API Key 1' },
        { id: 'key2', name: 'API Key 2' },
      ];
      
      supabase.from.mockReturnValueOnce({
        select: jest.fn().mockReturnThis(),
        eq: jest.fn().mockReturnThis(),
        order: jest.fn().mockResolvedValueOnce({
          data: mockApiKeys,
          error: null,
        }),
      });
      
      const result = await listApiKeys('user123');
      
      expect(supabase.from).toHaveBeenCalledWith('api_keys');
      expect(result).toEqual({ data: mockApiKeys, error: null });
    });
    
    it('deleteApiKey calls supabase.from("api_keys") with correct parameters', async () => {
      const mockResponse = { data: null, error: null };
      
      supabase.from.mockReturnValueOnce({
        delete: jest.fn().mockReturnThis(),
        eq: jest.fn().mockReturnThis(),
        eq: jest.fn().mockResolvedValueOnce(mockResponse),
      });
      
      const result = await deleteApiKey('key123', 'user123');
      
      expect(supabase.from).toHaveBeenCalledWith('api_keys');
      expect(result).toEqual(mockResponse);
    });
  });
  
  describe('Job Functions', () => {
    it('createJob calls supabase.from("jobs") with correct parameters', async () => {
      const mockJob = { 
        id: 'job123', 
        name: 'Test Job',
        status: 'pending',
      };
      
      supabase.from.mockReturnValueOnce({
        insert: jest.fn().mockReturnThis(),
        select: jest.fn().mockReturnThis(),
        single: jest.fn().mockResolvedValueOnce({
          data: mockJob,
          error: null,
        }),
      });
      
      const jobData = { name: 'Test Job', model: 'gpt-4', prompt: 'Test prompt' };
      const result = await createJob('user123', jobData);
      
      expect(supabase.from).toHaveBeenCalledWith('jobs');
      expect(result).toEqual({ data: mockJob, error: null });
    });
    
    it('listJobs calls supabase.from("jobs") with correct parameters', async () => {
      const mockJobs = [
        { id: 'job1', name: 'Job 1' },
        { id: 'job2', name: 'Job 2' },
      ];
      
      supabase.from.mockReturnValueOnce({
        select: jest.fn().mockReturnThis(),
        eq: jest.fn().mockReturnThis(),
        order: jest.fn().mockResolvedValueOnce({
          data: mockJobs,
          error: null,
        }),
      });
      
      const result = await listJobs('user123');
      
      expect(supabase.from).toHaveBeenCalledWith('jobs');
      expect(result).toEqual({ data: mockJobs, error: null });
    });
    
    it('getJobDetails calls supabase.from("jobs") with correct parameters', async () => {
      const mockJob = { id: 'job123', name: 'Test Job' };
      
      supabase.from.mockReturnValueOnce({
        select: jest.fn().mockReturnThis(),
        eq: jest.fn().mockReturnThis(),
        single: jest.fn().mockResolvedValueOnce({
          data: mockJob,
          error: null,
        }),
      });
      
      const result = await getJobDetails('job123');
      
      expect(supabase.from).toHaveBeenCalledWith('jobs');
      expect(result).toEqual({ data: mockJob, error: null });
    });
  });
});
