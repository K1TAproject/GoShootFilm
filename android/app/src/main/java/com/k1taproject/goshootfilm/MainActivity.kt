package com.k1taproject.goshootfilm

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.k1taproject.goshootfilm.ui.GsfBg
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfSurface
import com.k1taproject.goshootfilm.ui.GsfTheme
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.screens.AlbumsScreen
import com.k1taproject.goshootfilm.ui.screens.CollectionDetailScreen
import com.k1taproject.goshootfilm.ui.screens.EntityDetailScreen
import com.k1taproject.goshootfilm.ui.screens.EquipmentScreen
import com.k1taproject.goshootfilm.ui.screens.FilmsScreen
import com.k1taproject.goshootfilm.ui.screens.HomeScreen
import com.k1taproject.goshootfilm.ui.screens.PairScreen
import com.k1taproject.goshootfilm.ui.screens.RollsScreen
import com.k1taproject.goshootfilm.ui.screens.SettingsScreen

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WindowCompat.setDecorFitsSystemWindows(window, false)
        window.statusBarColor = android.graphics.Color.parseColor("#080B10")
        window.navigationBarColor = android.graphics.Color.parseColor("#080B10")
        setContent {
            GsfTheme {
                Box(Modifier.fillMaxSize().background(GsfBg)) {
                    GsfApp()
                }
            }
        }
    }
}

@Composable
fun GsfApp(vm: GsfViewModel = viewModel()) {
    val settings by vm.settings.collectAsStateWithLifecycle()
    if (settings.serverUrl.isBlank()) {
        PairScreen(vm)
    } else {
        MainScaffold(vm)
    }
}

private data class Tab(val route: String, val label: String)

private val tabs = listOf(
    Tab("home", "Home"),
    Tab("equipment", "Equipment"),
    Tab("films", "Films"),
    Tab("rolls", "Rolls"),
    Tab("albums", "Albums"),
)

@Composable
private fun MainScaffold(vm: GsfViewModel) {
    val navController = rememberNavController()
    val backStack by navController.currentBackStackEntryAsState()
    val current = backStack?.destination?.route
    val root = tabs.any { it.route == current }

    Scaffold(
        containerColor = GsfBg,
        bottomBar = {
            if (root) {
                NavigationBar(containerColor = GsfSurface, tonalElevation = 0.dp) {
                    tabs.forEach { tab ->
                        val selected = current == tab.route
                        NavigationBarItem(
                            selected = selected,
                            onClick = {
                                navController.navigate(tab.route) {
                                    popUpTo(navController.graph.findStartDestination().id) {
                                        saveState = true
                                    }
                                    launchSingleTop = true
                                    restoreState = true
                                }
                            },
                            icon = {
                                Box(
                                    modifier = Modifier
                                        .size(if (selected) 8.dp else 6.dp)
                                        .clip(CircleShape)
                                        .background(if (selected) GsfPurple else GsfMuted.copy(alpha = 0.5f)),
                                )
                            },
                            label = {
                                Text(
                                    text = tab.label,
                                    color = if (selected) GsfPurple else GsfMuted,
                                )
                            },
                            colors = NavigationBarItemDefaults.colors(
                                indicatorColor = Color.Transparent,
                            ),
                        )
                    }
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = "home",
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            composable("home") {
                HomeScreen(
                    vm = vm,
                    onSettings = { navController.navigate("settings") },
                    onRoll = { navController.navigate("roll/$it") },
                    onAlbum = { navController.navigate("album/$it") },
                    onOpenTab = { route -> navController.navigate(route) },
                )
            }
            composable("equipment") {
                EquipmentScreen(vm) { type, id ->
                    navController.navigate("entity/$type/$id")
                }
            }
            composable("films") { FilmsScreen(vm) }
            composable("rolls") {
                RollsScreen(vm) { navController.navigate("roll/$it") }
            }
            composable("albums") {
                AlbumsScreen(vm) { navController.navigate("album/$it") }
            }
            composable("settings") {
                SettingsScreen(vm) { navController.popBackStack() }
            }
            composable("roll/{id}") { entry ->
                CollectionDetailScreen(
                    vm = vm,
                    kind = "roll",
                    syncId = entry.arguments?.getString("id").orEmpty(),
                    onBack = { navController.popBackStack() },
                )
            }
            composable("album/{id}") { entry ->
                CollectionDetailScreen(
                    vm = vm,
                    kind = "digital_album",
                    syncId = entry.arguments?.getString("id").orEmpty(),
                    onBack = { navController.popBackStack() },
                )
            }
            composable("entity/{type}/{id}") { entry ->
                EntityDetailScreen(
                    vm = vm,
                    type = entry.arguments?.getString("type").orEmpty(),
                    syncId = entry.arguments?.getString("id").orEmpty(),
                    onBack = { navController.popBackStack() },
                )
            }
        }
    }
}
