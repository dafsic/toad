import { useEffect, useState } from 'react'

export function LoginPage() {
    const [code, setCode] = useState<string>('')
    const [loading, setLoading] = useState(true)
    const [error, setError] = useState<string>('')

    useEffect(() => {
        // 1. 请求验证码
        fetch('/api/auth/request', { method: 'POST' })
            .then((r) => r.json())
            .then((data: { code: string }) => {
                setCode(data.code)
                setLoading(false)

                // 2. 建立 SSE 连接等待验证
                const es = new EventSource(`/api/auth/wait/${data.code}`)

                es.onmessage = (e) => {
                    try {
                        const { token } = JSON.parse(e.data)
                        // 3. 设置 cookie 并跳转
                        document.cookie = `auth_token=${token}; path=/; max-age=28800` // 8小时
                        window.location.href = '/'
                    } catch (err) {
                        setError('登录失败，请刷新页面重试')
                    } finally {
                        es.close()
                    }
                }

                es.onerror = () => {
                    es.close()
                    setError('连接超时，请刷新页面重试')
                }
            })
            .catch(() => {
                setLoading(false)
                setError('获取验证码失败，请刷新页面')
            })
    }, [])

    if (loading) {
        return (
            <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-gray-900 to-gray-800">
                <div className="text-white text-xl">加载中...</div>
            </div>
        )
    }

    return (
        <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-gray-900 to-gray-800 p-4">
            <div className="bg-white rounded-lg shadow-2xl p-8 max-w-md w-full">
                <div className="text-center mb-8">
                    <h1 className="text-3xl font-bold text-gray-800 mb-2">🐸 Toad Grid Bot</h1>
                    <p className="text-gray-600">Telegram 身份验证</p>
                </div>

                {error ? (
                    <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
                        <p className="text-red-800 text-sm">{error}</p>
                    </div>
                ) : (
                    <>
                        <div className="bg-blue-50 border border-blue-200 rounded-lg p-6 mb-6">
                            <p className="text-gray-700 mb-4 text-sm">
                                请在 Telegram 中给 Bot 发送以下命令：
                            </p>
                            <div className="bg-white rounded border border-gray-300 p-4 font-mono text-center">
                                <span className="text-gray-600 text-sm">/login </span>
                                <span className="text-2xl font-bold text-blue-600">{code}</span>
                            </div>
                        </div>

                        <div className="text-center">
                            <div className="inline-flex items-center text-gray-600 text-sm">
                                <svg
                                    className="animate-spin h-4 w-4 mr-2"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                >
                                    <circle
                                        className="opacity-25"
                                        cx="12"
                                        cy="12"
                                        r="10"
                                        stroke="currentColor"
                                        strokeWidth="4"
                                    />
                                    <path
                                        className="opacity-75"
                                        fill="currentColor"
                                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                                    />
                                </svg>
                                等待 Telegram 验证...
                            </div>
                            <p className="text-xs text-gray-500 mt-4">验证码 5 分钟内有效</p>
                        </div>
                    </>
                )}
            </div>
        </div>
    )
}
