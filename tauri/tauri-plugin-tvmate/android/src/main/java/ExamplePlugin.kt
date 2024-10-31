package com.plugin.tvmate

import android.app.Activity
import android.content.Intent
import android.view.View
import android.view.Window
import android.view.WindowManager
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke

@InvokeArg
internal class ShareArgs {
    var title: String? = null
    lateinit var url: String
}


@TauriPlugin
class ExamplePlugin(private val activity: Activity): Plugin(activity) {
    var isFullScreen = false;

    init {
        val windowInsetsController =
            WindowCompat.getInsetsController(activity.window, activity.window.decorView)
        // Configure the behavior of the hidden system bars.
        windowInsetsController.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE

        // Add a listener to update the behavior of the toggle fullscreen button when
        // the system bars are hidden or revealed.
        ViewCompat.setOnApplyWindowInsetsListener(activity.window.decorView) { view, windowInsets ->
            // You can hide the caption bar even when the other system bars are visible.
            // To account for this, explicitly check the visibility of navigationBars()
            // and statusBars() rather than checking the visibility of systemBars().
            if (windowInsets.isVisible(WindowInsetsCompat.Type.navigationBars())
                || windowInsets.isVisible(WindowInsetsCompat.Type.statusBars())) {
                isFullScreen = false
            } else {
                isFullScreen = true
            }
            ViewCompat.onApplyWindowInsets(view, windowInsets)
        }
    }

    @Command
    fun isFullscreen(invoke: Invoke) {
        val result = JSObject()
        result.put("is_fullscreen", result)
        invoke.resolve(result)
    }

    @Command
    fun fullscreen(invoke: Invoke) {
        val windowInsetsController = WindowCompat.getInsetsController(activity.window,activity.window.decorView);
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars())
        val result = JSObject()
        result.put("is_fullscreen", result)
        invoke.resolve(result)
    }

    @Command
    fun exitFullscreen(invoke: Invoke) {
        val windowInsetsController = WindowCompat.getInsetsController(activity.window,activity.window.decorView);
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars())
        val result = JSObject()
        result.put("is_fullscreen", result)
        invoke.resolve(result)
    }

    @Command
    fun shareUrl(invoke: Invoke) {
        val shareArgs =   invoke.parseArgs(ShareArgs::class.java)
        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, shareArgs.url)
        }
        activity.startActivity(Intent.createChooser(shareIntent, "Share via"))
        invoke.resolve() 
    }
}
