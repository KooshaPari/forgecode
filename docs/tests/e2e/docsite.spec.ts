import { expect, test } from '@playwright/test'

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173'

test.describe('forgecode Documentation Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL)
  })

  test.describe('Homepage', () => {
    test('should load the homepage', async ({ page }) => {
      await expect(page).toHaveTitle(/forgecode/i)
      const response = await page.goto(BASE_URL)
      expect(response?.status()).toBeLessThan(400)
    })

    test('should have no console errors', async ({ page }) => {
      const errors: string[] = []
      page.on('console', (msg) => { if (msg.type() === 'error') errors.push(msg.text()) })
      await page.goto(BASE_URL)
      await page.waitForLoadState('networkidle')
      expect(errors).toHaveLength(0)
    })
  })

  test.describe('Navigation', () => {
    test('should have visible navigation', async ({ page }) => {
      await expect(page.locator('.VPNav')).toBeVisible()
    })
  })

  test.describe('Dark Mode', () => {
    test('should toggle dark mode', async ({ page }) => {
      await page.goto(BASE_URL)
      const themeButton = page.locator('[class*="theme"]').first()
      if (await themeButton.isVisible()) {
        await themeButton.click()
        await page.waitForTimeout(300)
      }
    })
  })

  test.describe('Localization', () => {
    test('should support zh-CN locale', async ({ page }) => {
      const response = await page.goto(`${BASE_URL}/zh-CN/`)
      expect(response?.status()).toBeLessThan(400)
    })
    test('should support zh-TW locale', async ({ page }) => {
      const response = await page.goto(`${BASE_URL}/zh-TW/`)
      expect(response?.status()).toBeLessThan(400)
    })
    test('should support fa locale', async ({ page }) => {
      const response = await page.goto(`${BASE_URL}/fa/`)
      expect(response?.status()).toBeLessThan(400)
    })
    test('should support fa-Latn locale', async ({ page }) => {
      const response = await page.goto(`${BASE_URL}/fa-Latn/`)
      expect(response?.status()).toBeLessThan(400)
    })
  })

  test.describe('Accessibility', () => {
    test('should have proper heading hierarchy', async ({ page }) => {
      await page.goto(BASE_URL)
      await expect(page.locator('h1')).toBeVisible()
    })
  })

  test.describe('Responsive Design', () => {
    test('should work on mobile viewport', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 })
      await page.goto(BASE_URL)
      await expect(page.locator('.VPNav')).toBeVisible()
    })
  })
})
